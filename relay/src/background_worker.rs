use crate::helper_functions::*;
use crate::info_collectors::cpu_ram::*;
use crate::info_collectors::disk::*;
use crate::info_collectors::gpu::*;
use crate::info_collectors::processes::*;
use crate::types::*;
use chrono::Local;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use sysinfo::Disks;
use sysinfo::{Components, System, Users};
use tokio::sync::RwLock;
use tokio::time;
use tokio_util::sync::CancellationToken;

pub async fn background_metrics_updater(
    metrics: Arc<RwLock<MetricsSnapshot>>,
    duration: Duration,
    processes_config: ProcessesConfig,
    smart_config: SmartConfig,
    token: CancellationToken,
) {
    let mut sys = match System::new_all() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to initialize sysinfo: {}", e);
            return;
        }
    };

    let mut cmp = match Components::new_with_refreshed_list() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to initialize sysinfo components: {}", e);
            return;
        }
    };

    let users = match Users::new_with_refreshed_list() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to initialize sysinfo users list: {}", e);
            return;
        }
    };

    let mut disks = match Disks::new_with_refreshed_list() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to initialize sysinfo Disks: {}", e);
            return;
        }
    };

    let sys_net = Path::new("/sys/class/net");
    sys.refresh_all();
    let mut interval = time::interval(duration);

    let mut last_rapl_state: Option<RaplState> = None;
    let mut last_disk_io_states: HashMap<String, DiskIoRawState> = HashMap::new();

    let (mut cached_smart_drives, mut smart_last_updated) = {
        let existing = metrics.read().await;
        (
            existing.storage.smart_drives.clone(),
            existing.storage.smart_last_updated.clone(),
        )
    };

    let smart_poll_interval = Duration::from_secs(smart_config.interval_secs.max(300));
    let mut last_smart_poll: Option<std::time::Instant> = smart_last_updated
        .as_ref()
        .and_then(|ts| chrono::DateTime::parse_from_rfc3339(ts).ok())
        .and_then(|dt| {
            let elapsed_secs = (Local::now() - dt.with_timezone(&Local)).num_seconds();
            if elapsed_secs >= 0 {
                std::time::Instant::now().checked_sub(Duration::from_secs(elapsed_secs as u64))
            } else {
                None
            }
        });

    // `None` means no usable NVIDIA GPU; GPU polling is then skipped entirely.
    let gpu_static_info = match parse_gpu_static_info() {
        Ok(info) => Some(info),
        Err(e) => {
            eprintln!("GPU metrics disabled: {}", e);
            None
        }
    };

    loop {
        tokio::select! {
            _ = token.cancelled() => {
                println!("Background metrics updater loop terminated gracefully.");
                break;
            }
            _ = interval.tick() => {
                sys.refresh_all();
                cmp.refresh(false);

                let now = std::time::Instant::now();

                let should_poll_smart = match last_smart_poll {
                    None => true,
                    Some(prev) => now.duration_since(prev) >= smart_poll_interval,
                };

                if should_poll_smart {
                    let cfg_clone = smart_config.clone();
                    if let Ok(drives) = tokio::task::spawn_blocking(move || collect_all_smart_info(&cfg_clone)).await {
                        cached_smart_drives = drives;
                        smart_last_updated = Some(Local::now().to_rfc3339());
                        last_smart_poll = Some(now);
                    }
                }

                // --- Fast Storage Metrics (runs every `duration` seconds) ---
                let (total_cap_gb, total_used_gb, total_avail_gb, mounts) = collect_mount_usage(&mut disks);
                let io_stats = collect_disk_io_rates(&mut last_disk_io_states);

                let storage = StorageSummary {
                    total_capacity_gb: total_cap_gb,
                    total_used_gb: total_used_gb,
                    total_available_gb: total_avail_gb,
                    mounts,
                    io_stats,
                    smart_drives: cached_smart_drives.clone(),
                    smart_last_updated: smart_last_updated.clone(),
                };

                let current_energy = match read_total_cpu_energy_uj() {
                                    Ok(energy) => Some(energy),
                                    Err(_) => None,
                                };
                let mut calculated_watts = 0.0;
                if let (Some(e_current), Some(state)) = (current_energy, last_rapl_state) {
                                    let delta_time = now.duration_since(state.timestamp).as_secs_f64();

                                    let delta_uj = if e_current >= state.energy_uj {
                                        e_current - state.energy_uj
                                    } else {
                                        (u64::MAX - state.energy_uj) + e_current // Handle counter rollover
                                    };

                                    if delta_time > 0.0 {
                                        calculated_watts = (delta_uj as f64 / 1_000_000.0) / delta_time;
                                    }
                                }

                                // Update the state persistent frame
                                if let Some(e_current) = current_energy {
                                    last_rapl_state = Some(RaplState {
                                        energy_uj: e_current,
                                        timestamp: now,
                                    });
                                }

                let mut network_interfaces: NetworkInterfaces = Vec::new();

                if let Ok(entries) = fs::read_dir(sys_net) {
                    for entry in entries.flatten() {
                        let iface_name = entry.file_name().into_string().unwrap();
                        let tx_bytes = read_sys_file(&iface_name, "statistics/tx_bytes");
                        let rx_bytes = read_sys_file(&iface_name, "statistics/rx_bytes");
                        let mut speed = read_sys_file(&iface_name, "speed");

                        if speed == "0" || speed.contains('-') {
                            speed = "N/A (Virtual/Down)".to_string();
                        }

                        network_interfaces.push(NetworkInterfaceInfo {
                            name: iface_name,
                            tx_bytes,
                            rx_bytes,
                            speed_mbps: speed,
                        });
                    }
                }

                let cpus = sys.cpus();
                let avg_cpu = if cpus.is_empty() {
                    0.0
                } else {
                    cpus.iter().map(|c| c.usage()).sum::<f32>() / cpus.len() as f32
                };
                let avg_cpu_frequency = if cpus.is_empty() {
                    0.0
                } else {
                    cpus.iter().map(|c| c.frequency() as f64).sum::<f64>() / cpus.len() as f64
                };

                let ram_sticks = get_populated_ram().unwrap_or_else(|e| {
                    eprintln!("Warning collecting RAM info: {}", e);
                    Vec::new() // Fallback to an empty vector if udevadm fails
                });

                let total_mem_gb = sys.total_memory() as f64 / 1_073_741_824.0;
                let used_mem_gb = sys.used_memory() as f64 / 1_073_741_824.0;
                let free_mem_gb = sys.free_memory() as f64 / 1_073_741_824.0;
                let available_mem_gb = sys.available_memory() as f64 / 1_073_741_824.0;
                let swap_total_mem_gb = sys.total_swap() as f64 / 1_073_741_824.0;
                let swap_used_mem_gb = sys.used_swap() as f64 / 1_073_741_824.0;
                let swap_free_mem_gb = sys.free_swap() as f64 / 1_073_741_824.0;

                let readings = collect_temperatures(&mut cmp);

                let processes = get_top_processes(&sys, &users, processes_config);

                let uptime = System::uptime().unwrap_or(0);
                let hostname = System::host_name().unwrap_or_else(|_| "unknown".to_string());
                let timestamp = Local::now().to_rfc3339();

                let gpu = match &gpu_static_info {
                    Some(static_info) => GPUInfo {
                        static_info: static_info.clone(),
                        dynamic_info: parse_gpu_dynamic_info().unwrap_or_else(|e| {
                            eprintln!("Warning collecting GPU info: {}", e);
                            Default::default()
                        }),
                    },
                    None => GPUInfo {
                        static_info: Default::default(),
                        dynamic_info: Default::default(),
                    },
                };

                let new_snapshot = MetricsSnapshot {
                    hostname,
                    cpu: CPUInfo {
                        name: cpus.first().map_or("unknown".to_string(), |c| c.brand().to_string()),
                        usage: cpus.iter().map(|c| (c.usage() * 100.0).trunc() / 100.0).collect(),
                        avg_usage: avg_cpu,
                        frequency: cpus.iter().map(|c| ((c.frequency() as f64 / 1_000.0) * 100.0).trunc() / 100.0).collect(),
                        avg_frequency: avg_cpu_frequency / 1_000.0,
                        package_watts: calculated_watts
                    },
                    ram_sticks,
                    memory: MemoryInfo {
                        total_gb: total_mem_gb,
                        used_gb: used_mem_gb,
                        free_gb: free_mem_gb,
                        available_gb: available_mem_gb,
                    },
                    swap: SwapInfo {
                        total_gb: swap_total_mem_gb,
                        used_gb: swap_used_mem_gb,
                        free_gb: swap_free_mem_gb,
                    },
                    gpu,
                    storage,
                    temp_sensor_readings:readings,
                    network_interfaces,
                    processes,
                    uptime_secs: uptime,
                    polling_interval_secs: duration.as_secs() as u64,
                    timestamp,
                };

                let mut write_guard = metrics.write().await;
                *write_guard = new_snapshot;
            }
        }
    }
}
