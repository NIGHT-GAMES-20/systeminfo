use crate::helper_functions::*;
use crate::types::*;
use chrono::Local;
use regex::Regex;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::str;
use std::sync::Arc;
use std::time::Duration;
use sysinfo::{Components, System};
use tokio::sync::RwLock;
use tokio::time;
use tokio_util::sync::CancellationToken;

pub fn parse_gpu_static_info() -> Result<GPUStaticInfo, String> {
    let output = Command::new("nvidia-smi")
        .arg("-q")
        .output()
        .map_err(|e| format!("Static query invocation failed: {}", e))?;

    if !output.status.success() {
        return Err(str::from_utf8(&output.stderr)
            .unwrap_or("NV-SMI static error")
            .to_string());
    }

    let payload = str::from_utf8(&output.stdout)
        .map_err(|e| format!("Failed to parse static stdout as valid UTF-8: {}", e))?;

    let r_str = |pat: &str| Regex::new(pat).unwrap();

    let pull_str = |re: &Regex| {
        re.captures(payload)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_default()
    };

    let pull_f64 = |re: &Regex| {
        re.captures(payload)
            .and_then(|c| c.get(1))
            .and_then(|m| m.as_str().trim().parse::<f64>().ok())
            .unwrap_or(0.0)
    };

    let pull_u32 = |re: &Regex| {
        re.captures(payload)
            .and_then(|c| c.get(1))
            .and_then(|m| m.as_str().trim().parse::<u32>().ok())
            .unwrap_or(0)
    };

    let mut static_info = GPUStaticInfo::default();

    static_info.kmd_version = pull_str(&r_str(r"KMD Version\s+:\s+(.+)"));
    static_info.umd_version = pull_str(&r_str(r"CUDA UMD Version\s+:\s+(.+)"));
    static_info.product_name = pull_str(&r_str(r"Product Name\s+:\s+(.+)"));
    static_info.vbios_version = pull_str(&r_str(r"VBIOS Version\s+:\s+(.+)"));
    static_info.gpu_uuid = pull_str(&r_str(r"GPU UUID\s+:\s+(.+)"));
    static_info.gpu_pdi = pull_str(&r_str(r"GPU PDI\s+:\s+(.+)"));

    static_info.pcie_info.bus = pull_str(&r_str(r"Bus\s+:\s+(.+)"));
    static_info.pcie_info.device_id = pull_str(&r_str(r"Device Id\s+:\s+(.+)"));
    static_info.pcie_info.bus_id = pull_str(&r_str(r"Bus Id\s+:\s+(.+)"));

    // Robust regex: matches "Current" anywhere after "PCIe Generation"
    static_info.pcie_info.current_pcie_gen =
        pull_str(&r_str(r"PCIe Generation[\s\S]*?Current\s+:\s+(.+)"));
    static_info.pcie_info.current_link_width =
        pull_str(&r_str(r"Link Width[\s\S]*?Current\s+:\s+(.+)"));

    static_info.power_info.min_limit = pull_f64(&r_str(r"Min Power Limit\s+:\s+([\d\.]+)\s+W"));
    static_info.power_info.max_limit = pull_f64(&r_str(r"Max Power Limit\s+:\s+([\d\.]+)\s+W"));

    // Robust regex: matches the specific clock type anywhere after "Max Clocks"
    static_info.clock_info.max_graphics =
        pull_u32(&r_str(r"Max Clocks[\s\S]*?Graphics\s+:\s+(\d+)\s+MHz"));
    static_info.clock_info.max_sm = pull_u32(&r_str(r"Max Clocks[\s\S]*?SM\s+:\s+(\d+)\s+MHz"));
    static_info.clock_info.max_memory =
        pull_u32(&r_str(r"Max Clocks[\s\S]*?Memory\s+:\s+(\d+)\s+MHz"));
    static_info.clock_info.max_video =
        pull_u32(&r_str(r"Max Clocks[\s\S]*?Video\s+:\s+(\d+)\s+MHz"));

    Ok(static_info)
}

pub fn parse_gpu_dynamic_info() -> Result<GPUDynamicInfo, String> {
    // --- CALL 1: Standard comprehensive status metrics ---
    let output_q = Command::new("nvidia-smi")
        .arg("-q")
        .output()
        .map_err(|e| format!("Dynamic status query failed: {}", e))?;

    if !output_q.status.success() {
        return Err(str::from_utf8(&output_q.stderr)
            .unwrap_or("NV-SMI status query error")
            .to_string());
    }
    let payload_q = str::from_utf8(&output_q.stdout)
        .map_err(|e| format!("Failed to parse status payload as valid UTF-8: {}", e))?;

    // --- CALL 2: Dedicated power telemetry mapping ---
    let output_p = Command::new("nvidia-smi")
        .arg("-q")
        .arg("-d")
        .arg("POWER")
        .output()
        .map_err(|e| format!("Dynamic power query failed: {}", e))?;

    if !output_p.status.success() {
        return Err(str::from_utf8(&output_p.stderr)
            .unwrap_or("NV-SMI power query error")
            .to_string());
    }
    let payload_p = str::from_utf8(&output_p.stdout)
        .map_err(|e| format!("Failed to parse power payload as valid UTF-8: {}", e))?;

    let r_str = |pat: &str| Regex::new(pat).unwrap();

    let pull_str = |re: &Regex, text: &str| {
        re.captures(text)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_default()
    };

    let pull_f64 = |re: &Regex, text: &str| {
        re.captures(text)
            .and_then(|c| c.get(1))
            .and_then(|m| m.as_str().trim().parse::<f64>().ok())
            .unwrap_or(0.0)
    };

    let pull_u32 = |re: &Regex, text: &str| {
        re.captures(text)
            .and_then(|c| c.get(1))
            .and_then(|m| m.as_str().trim().parse::<u32>().ok())
            .unwrap_or(0)
    };

    let parse_mem_to_gb = |pat: &str, text: &str| -> f64 {
        r_str(pat)
            .captures(text)
            .and_then(|c| c.get(1))
            .and_then(|m| m.as_str().trim().parse::<f64>().ok())
            .map(|val| val / 1024.0)
            .unwrap_or(0.0)
    };

    let mut dynamic_info = GPUDynamicInfo::default();

    dynamic_info.pcie_throughput.tx_throughput =
        pull_f64(&r_str(r"Tx Throughput\s+:\s+([\d\.]+)\s+KB/s"), payload_q);
    dynamic_info.pcie_throughput.rx_throughput =
        pull_f64(&r_str(r"Rx Throughput\s+:\s+([\d\.]+)\s+KB/s"), payload_q);

    dynamic_info.fan_speed = pull_u32(&r_str(r"Fan Speed\s+:\s+(\d+)\s*%"), payload_q);
    dynamic_info.performance_state = pull_str(&r_str(r"Performance State\s+:\s+(.+)"), payload_q);
    dynamic_info.current_temp =
        pull_f64(&r_str(r"GPU Current Temp\s+:\s+([\d\.]+)\s*C"), payload_q);

    // Robust memory parsing: ignores line order completely
    dynamic_info.fb_memory.total_gb = parse_mem_to_gb(
        r"FB Memory Usage[\s\S]*?Total\s+:\s+([\d\.]+)\s+MiB",
        payload_q,
    );
    dynamic_info.fb_memory.reserved_gb = parse_mem_to_gb(
        r"FB Memory Usage[\s\S]*?Reserved\s+:\s+([\d\.]+)\s+MiB",
        payload_q,
    );
    dynamic_info.fb_memory.used_gb = parse_mem_to_gb(
        r"FB Memory Usage[\s\S]*?Used\s+:\s+([\d\.]+)\s+MiB",
        payload_q,
    );
    dynamic_info.fb_memory.free_gb = parse_mem_to_gb(
        r"FB Memory Usage[\s\S]*?Free\s+:\s+([\d\.]+)\s+MiB",
        payload_q,
    );

    dynamic_info.bar1_memory.total_gb = parse_mem_to_gb(
        r"BAR1 Memory Usage[\s\S]*?Total\s+:\s+([\d\.]+)\s+MiB",
        payload_q,
    );
    dynamic_info.bar1_memory.used_gb = parse_mem_to_gb(
        r"BAR1 Memory Usage[\s\S]*?Used\s+:\s+([\d\.]+)\s+MiB",
        payload_q,
    );
    dynamic_info.bar1_memory.free_gb = parse_mem_to_gb(
        r"BAR1 Memory Usage[\s\S]*?Free\s+:\s+([\d\.]+)\s+MiB",
        payload_q,
    );

    // Robust utilization parsing: uses exact field names as they appear in nvidia-smi
    dynamic_info.utilization.gpu = pull_f64(
        &r_str(r"Utilization[\s\S]*?GPU\s+:\s+([\d\.]+)\s*%"),
        payload_q,
    );
    dynamic_info.utilization.memory = pull_f64(
        &r_str(r"Utilization[\s\S]*?Memory\s+:\s+([\d\.]+)\s*%"),
        payload_q,
    );
    dynamic_info.utilization.encoder = pull_f64(
        &r_str(r"Utilization[\s\S]*?Encoder\s+:\s+([\d\.]+)\s*%"),
        payload_q,
    );
    dynamic_info.utilization.decoder = pull_f64(
        &r_str(r"Utilization[\s\S]*?Decoder\s+:\s+([\d\.]+)\s*%"),
        payload_q,
    );
    dynamic_info.utilization.jpeg = pull_f64(
        &r_str(r"Utilization[\s\S]*?JPEG\s+:\s+([\d\.]+)\s*%"),
        payload_q,
    );
    dynamic_info.utilization.ofa = pull_f64(
        &r_str(r"Utilization[\s\S]*?OFA\s+:\s+([\d\.]+)\s*%"),
        payload_q,
    );

    // Robust clock parsing
    dynamic_info.clock.graphics = pull_f64(
        &r_str(r"Clocks[\s\S]*?Graphics\s+:\s+([\d\.]+)\s+MHz"),
        payload_q,
    );
    dynamic_info.clock.sm = pull_f64(&r_str(r"Clocks[\s\S]*?SM\s+:\s+([\d\.]+)\s+MHz"), payload_q);
    dynamic_info.clock.memory = pull_f64(
        &r_str(r"Clocks[\s\S]*?Memory\s+:\s+([\d\.]+)\s+MHz"),
        payload_q,
    );
    dynamic_info.clock.video = pull_f64(
        &r_str(r"Clocks[\s\S]*?Video\s+:\s+([\d\.]+)\s+MHz"),
        payload_q,
    );

    // Parse the live power usage exactly out of the dedicated Call 2 payload
    let main_power = r_str(r"Average Power Draw\s+:\s+([\d\.]+)\s+W")
        .captures(payload_p)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse::<f64>().ok());

    dynamic_info.power_usage = match main_power {
        Some(val) => val,
        None => r_str(r"(?s)Power Samples.*?Avg\s+:\s+([\d\.]+)\s+W")
            .captures(payload_p)
            .and_then(|c| c.get(1))
            .and_then(|m| m.as_str().parse::<f64>().ok())
            .unwrap_or(0.0),
    };

    Ok(dynamic_info)
}

pub fn collect_temperatures(components: &mut Components) -> Vec<TempSensorReading> {
    // Refresh only the temperature values of existing discovered components
    for comp in components.iter_mut() {
        comp.refresh();
    }

    components
        .iter()
        .map(|comp| {
            let raw_label = comp.label().to_string();

            // Split "k10temp Tctl" into ("k10temp", "Tctl")
            let (chip, sensor) = raw_label
                .split_once(' ')
                .map(|(c, s)| (c.to_string(), s.to_string()))
                .unwrap_or_else(|| ("unknown".to_string(), raw_label.clone()));

            TempSensorReading {
                id: comp.id().unwrap_or("unknown").to_string(),
                chip,
                sensor,
                raw_label,
                temp_celsius: clean_float(comp.temperature()),
                max_celsius: clean_float(comp.max()),
                crit_celsius: clean_float(comp.critical()),
            }
        })
        .collect()
}

pub fn get_populated_ram() -> Result<Vec<RamStick>, String> {
    let output = Command::new("udevadm")
        .arg("info")
        .arg("-e")
        .output()
        .map_err(|e| format!("Failed to execute udevadm: {}", e))?;

    if !output.status.success() {
        return Err("udevadm command returned a non-zero exit status".to_string());
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let mut ram_slots_map: BTreeMap<u32, RamStick> = BTreeMap::new();

    for line in stdout_str.lines() {
        if !line.contains("MEMORY_DEVICE_") {
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            let clean_value = value.trim();

            if clean_value == "Unknown" || clean_value.is_empty() {
                continue;
            }

            let tokens: Vec<&str> = key.split('_').collect();
            if tokens.len() < 3 {
                continue;
            }

            let slot_str = tokens.get(2).copied().unwrap_or("");
            if let Ok(slot_num) = slot_str.parse::<u32>() {
                let stick = ram_slots_map.entry(slot_num).or_insert(RamStick {
                    slot: slot_num,
                    manufacturer: None,
                    part_number: None,
                });

                if key.contains("MANUFACTURER") {
                    stick.manufacturer = Some(clean_value.to_string());
                } else if key.contains("PART_NUMBER") {
                    stick.part_number = Some(clean_value.to_string());
                }
            }
        }
    }

    let populated_sticks: Vec<RamStick> = ram_slots_map
        .into_values()
        .filter(|stick| stick.manufacturer.is_some() || stick.part_number.is_some())
        .collect();

    Ok(populated_sticks)
}
pub async fn background_metrics_updater(
    metrics: Arc<RwLock<MetricsSnapshot>>,
    duration: Duration,
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

    let sys_net = Path::new("/sys/class/net");
    sys.refresh_all();
    let mut interval = time::interval(duration);

    let gpu_static_info =
        parse_gpu_static_info().expect("Critical: Failed to read static GPU hardware properties");

    loop {
        tokio::select! {
            _ = token.cancelled() => {
                println!("🛑 Background metrics updater loop terminated gracefully.");
                break;
            }
            _ = interval.tick() => {
                sys.refresh_all();
                cmp.refresh(false);

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

                let uptime = System::uptime().unwrap_or(0);
                let hostname = System::host_name().unwrap_or_else(|_| "unknown".to_string());
                let timestamp = Local::now().to_rfc3339();

                let new_snapshot = MetricsSnapshot {
                    hostname,
                    cpu: CPUInfo {
                        name: cpus.first().map_or("unknown".to_string(), |c| c.brand().to_string()),
                        usage: cpus.iter().map(|c| (c.usage() * 100.0).trunc() / 100.0).collect(),
                        avg_usage: avg_cpu,
                        frequency: cpus.iter().map(|c| ((c.frequency() as f64 / 1_000.0) * 100.0).trunc() / 100.0).collect(),
                        avg_frequency: avg_cpu_frequency / 1_000.0
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
                    gpu: GPUInfo {
                      static_info: gpu_static_info.clone(),
                      dynamic_info: parse_gpu_dynamic_info().expect("Critical: Failed to read Dynamic GPU hardware properties"),
                    },
                    temp_sensor_readings:readings,
                    network_interfaces,
                    uptime_secs: uptime,
                    polling_interval_secs: duration.as_secs() as u8,
                    timestamp,
                };

                let mut write_guard = metrics.write().await;
                *write_guard = new_snapshot;
            }
        }
    }
}
