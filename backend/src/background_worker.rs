use crate::helper_functions::*;
use crate::types::*;
use chrono::Local;
use regex::Regex;
use serde_json::Value;
use std::collections::BTreeMap;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::str;
use std::sync::Arc;
use std::time::Duration;
use sysinfo::Disks;
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

    let populated_sticks: Vec<RamStick> = ram_slots_map.into_values().collect();

    Ok(populated_sticks)
}

const RAPL_BASE_PATH: &str = "/sys/class/powercap/intel-rapl";

/// Reads the total energy across all detected CPU packages in microjoules
pub fn read_total_cpu_energy_uj() -> Result<u64, String> {
    let mut total_energy = 0u64;
    let mut zones_found = 0;

    // Scan for all top-level package paths (e.g., intel-rapl:0, intel-rapl:1)
    if let Ok(entries) = fs::read_dir(RAPL_BASE_PATH) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(dirname) = path.file_name().and_then(|n| n.to_str()) {
                // Top-level packages match intel-rapl:X (subzones like intel-rapl:0:0 are cores/dram)
                if dirname.starts_with("intel-rapl:") && dirname.matches(':').count() == 1 {
                    let energy_path = path.join("energy_uj");
                    if let Ok(content) = fs::read_to_string(energy_path) {
                        if let Ok(energy) = content.trim().parse::<u64>() {
                            total_energy += energy;
                            zones_found += 1;
                        }
                    }
                }
            }
        }
    }

    if zones_found == 0 {
        return Err(
            "No active RAPL packages detected. Verify root permissions / hardware compatibility."
                .to_string(),
        );
    }

    Ok(total_energy)
}

fn discover_smart_devices(
    extra_args_map: &BTreeMap<String, Vec<String>>,
) -> Vec<(String, Vec<String>)> {
    let mut devices: BTreeMap<String, Vec<String>> = BTreeMap::new();

    // 1. Auto-scan via smartctl
    if let Ok(output) = Command::new("smartctl")
        .args(["--scan-open", "--json"])
        .output()
    {
        if let Ok(json) = serde_json::from_slice::<Value>(&output.stdout) {
            if let Some(arr) = json.get("devices").and_then(|d| d.as_array()) {
                for dev in arr {
                    if let Some(name) = dev.get("name").and_then(|n| n.as_str()) {
                        let mut args = Vec::new();
                        // If user provided explicit overrides for this device, use them;
                        // otherwise trust smartctl's detected type (e.g., sat, nvme, scsi)
                        if let Some(custom) = extra_args_map.get(name) {
                            args.extend(custom.clone());
                        } else if let Some(dev_type) = dev.get("type").and_then(|t| t.as_str()) {
                            args.push("-d".to_string());
                            args.push(dev_type.to_string());
                        }
                        devices.insert(name.to_string(), args);
                    }
                }
            }
        }
    }

    // 2. Ensure any manually configured devices (like hidden USB bridges) are always queried
    for (dev_path, custom_args) in extra_args_map {
        devices.insert(dev_path.clone(), custom_args.clone());
    }

    devices.into_iter().collect()
}

/// Parses a single drive's SMART telemetry using `smartctl --json -a`
pub fn query_single_drive_smart(device: &str, extra_args: &[String]) -> DriveSmartInfo {
    let mut info = DriveSmartInfo {
        device: device.to_string(),
        ..Default::default()
    };

    let mut cmd = Command::new("smartctl");
    cmd.arg("--json").arg("-a");
    for arg in extra_args {
        cmd.arg(arg);
    }
    cmd.arg(device);

    let output = match cmd.output() {
        Ok(o) => o,
        Err(e) => {
            info.error = Some(format!("Failed to execute smartctl: {}", e));
            return info;
        }
    };

    // Note: smartctl uses bitmask exit codes (e.g., bit 0/1 = CLI error, bit 2+ = SMART threshold/log warnings).
    // Even on non-zero exits, stdout almost always contains valid JSON with exact error messages.
    let json: Value = match serde_json::from_slice(&output.stdout) {
        Ok(v) => v,
        Err(e) => {
            info.error = Some(format!(
                "Invalid JSON from smartctl (exit {:?}): {}",
                output.status.code(),
                e
            ));
            return info;
        }
    };

    // Capture any smartctl internal messages/errors (e.g. "Unknown USB bridge")
    if let Some(msgs) = json
        .pointer("/smartctl/messages")
        .and_then(|m| m.as_array())
    {
        let err_texts: Vec<&str> = msgs
            .iter()
            .filter_map(|m| m.get("string").and_then(|s| s.as_str()))
            .collect();
        if !err_texts.is_empty() {
            info.error = Some(err_texts.join("; "));
        }
    }

    info.protocol = json
        .pointer("/device/protocol")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();

    info.model_name = json
        .get("model_name")
        .or_else(|| json.get("scsi_model_name"))
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .trim()
        .to_string();

    info.serial_number = json
        .get("serial_number")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .trim()
        .to_string();

    info.firmware_version = json
        .get("firmware_version")
        .or_else(|| json.get("scsi_revision"))
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .trim()
        .to_string();

    info.capacity_bytes = json
        .pointer("/user_capacity/bytes")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    info.smart_passed = json
        .pointer("/smart_status/passed")
        .and_then(|v| v.as_bool());

    info.temperature_celsius = json
        .pointer("/temperature/current")
        .and_then(|v| v.as_f64());

    info.power_on_hours = json
        .pointer("/power_on_time/hours")
        .and_then(|v| v.as_u64());

    info.power_cycle_count = json.get("power_cycle_count").and_then(|v| v.as_u64());

    // NVMe percentage_used
    info.percentage_used = json
        .pointer("/nvme_smart_health_information_log/percentage_used")
        .and_then(|v| v.as_u64())
        .map(|v| v.min(100) as u8);

    // ATA/SATA SMART attributes table
    if let Some(table) = json
        .pointer("/ata_smart_attributes/table")
        .and_then(|v| v.as_array())
    {
        for attr in table {
            let id = attr.get("id").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            let name = attr
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string();
            let value = attr.get("value").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            let worst = attr.get("worst").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            let thresh = attr.get("thresh").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            let raw_value = attr
                .pointer("/raw/value")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let raw_string = attr
                .pointer("/raw/string")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            // Fallback SSD wear percentage from common ATA attributes (177 Wear_Leveling_Count, 231 SSD_Life_Left, 233 Media_Wearout_Indicator)
            if info.percentage_used.is_none() && matches!(id, 177 | 231 | 233) && value <= 100 {
                info.percentage_used = Some(100u8.saturating_sub(value as u8));
            }

            info.ata_attributes.push(SmartAttribute {
                id,
                name,
                value,
                worst,
                thresh,
                raw_value,
                raw_string,
            });
        }
    }

    info
}

pub fn collect_all_smart_info(config: &SmartConfig) -> Vec<DriveSmartInfo> {
    let targets = discover_smart_devices(&config.device_extra_args);
    targets
        .into_iter()
        .map(|(dev, args)| query_single_drive_smart(&dev, &args))
        .collect()
}

/// Collects mounted filesystem space (deduplicating bind mounts by device source)
pub fn collect_mount_usage(disks: &mut Disks) -> (f64, f64, f64, Vec<MountUsageInfo>) {
    disks.refresh(true);

    let mut seen_sources = HashSet::new();
    let mut total_cap: f64 = 0.0;
    let mut total_used: f64 = 0.0;
    let mut total_avail: f64 = 0.0;
    let mut mounts = Vec::new();

    for disk in disks.iter() {
        let fs_type = disk.file_system().to_string_lossy().to_string();
        // Skip pseudo/virtual filesystems
        if matches!(
            fs_type.as_str(),
            "tmpfs" | "devtmpfs" | "squashfs" | "overlay" | "proc" | "sysfs"
        ) {
            continue;
        }

        let total_bytes = disk.total_space();
        if total_bytes == 0 {
            continue;
        }

        let dev_name = disk.name().to_string_lossy().to_string();
        let avail_bytes = disk.available_space();
        let used_bytes = total_bytes.saturating_sub(avail_bytes);

        let total_gb: f64 = total_bytes as f64 / 1_073_741_824.0;
        let used_gb: f64 = used_bytes as f64 / 1_073_741_824.0;
        let available_gb: f64 = avail_bytes as f64 / 1_073_741_824.0;
        let usage_percent: f64 = (used_bytes as f64 / total_bytes as f64) * 100.0;

        // Only count each underlying block partition once toward system-wide totals
        if seen_sources.insert(dev_name.clone()) {
            total_cap += total_gb;
            total_used += used_gb;
            total_avail += available_gb;
        }

        mounts.push(MountUsageInfo {
            mount_point: disk.mount_point().to_string_lossy().to_string(),
            device_name: dev_name,
            file_system: fs_type,
            total_gb: (total_gb * 100.0_f64).trunc() / 100.0,
            used_gb: (used_gb * 100.0_f64).trunc() / 100.0,
            available_gb: (available_gb * 100.0_f64).trunc() / 100.0,
            usage_percent: (usage_percent * 100.0_f64).trunc() / 100.0,
            is_removable: disk.is_removable(),
        });
    }

    (
        (total_cap * 100.0_f64).trunc() / 100.0,
        (total_used * 100.0_f64).trunc() / 100.0,
        (total_avail * 100.0_f64).trunc() / 100.0,
        mounts,
    )
}

/// Reads `/proc/diskstats` and computes per-second MB/s and IOPS rates for whole physical drives
pub fn collect_disk_io_rates(
    prev_states: &mut HashMap<String, DiskIoRawState>,
) -> Vec<BlockDeviceIoStats> {
    let mut stats = Vec::new();
    let now = std::time::Instant::now();

    let Ok(content) = fs::read_to_string("/proc/diskstats") else {
        return stats;
    };

    for line in content.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 14 {
            continue;
        }

        let dev_name = cols[2];

        // Filter to whole physical disks only (exclude loop, ram, dm-, and partition numbers like sda1 or nvme0n1p1)
        let is_whole_disk = if dev_name.starts_with("nvme") {
            dev_name.contains('n') && !dev_name.contains('p')
        } else if dev_name.starts_with("sd")
            || dev_name.starts_with("vd")
            || dev_name.starts_with("hd")
        {
            !dev_name
                .chars()
                .last()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
        } else {
            false
        };

        if !is_whole_disk {
            continue;
        }

        let reads_completed = cols[3].parse::<u64>().unwrap_or(0);
        let sectors_read = cols[5].parse::<u64>().unwrap_or(0);
        let writes_completed = cols[7].parse::<u64>().unwrap_or(0);
        let sectors_written = cols[9].parse::<u64>().unwrap_or(0);

        let mut read_mb_per_sec: f64 = 0.0;
        let mut write_mb_per_sec: f64 = 0.0;
        let mut read_iops: f64 = 0.0;
        let mut write_iops: f64 = 0.0;

        if let Some(prev) = prev_states.get(dev_name) {
            let dt = now.duration_since(prev.timestamp).as_secs_f64();
            if dt > 0.0 {
                // Linux kernel /proc/diskstats always reports sectors in 512-byte units
                let delta_read_bytes = sectors_read.saturating_sub(prev.sectors_read) * 512;
                let delta_write_bytes = sectors_written.saturating_sub(prev.sectors_written) * 512;
                let delta_reads = reads_completed.saturating_sub(prev.reads_completed);
                let delta_writes = writes_completed.saturating_sub(prev.writes_completed);

                read_mb_per_sec = (delta_read_bytes as f64 / 1_048_576.0) / dt;
                write_mb_per_sec = (delta_write_bytes as f64 / 1_048_576.0) / dt;
                read_iops = delta_reads as f64 / dt;
                write_iops = delta_writes as f64 / dt;
            }
        }

        prev_states.insert(
            dev_name.to_string(),
            DiskIoRawState {
                sectors_read,
                sectors_written,
                reads_completed,
                writes_completed,
                timestamp: now,
            },
        );

        let total_read_gb: f64 = (sectors_read * 512) as f64 / 1_073_741_824.0;
        let total_written_gb: f64 = (sectors_written * 512) as f64 / 1_073_741_824.0;

        stats.push(BlockDeviceIoStats {
            device: dev_name.to_string(),
            read_mb_per_sec: (read_mb_per_sec * 100.0_f64).trunc() / 100.0,
            write_mb_per_sec: (write_mb_per_sec * 100.0_f64).trunc() / 100.0,
            read_iops: (read_iops * 10.0_f64).trunc() / 10.0,
            write_iops: (write_iops * 10.0_f64).trunc() / 10.0,
            total_read_gb: (total_read_gb * 100.0_f64).trunc() / 100.0,
            total_written_gb: (total_written_gb * 100.0_f64).trunc() / 100.0,
        });
    }

    stats
}

/// MAIN FUNCTION

pub async fn background_metrics_updater(
    metrics: Arc<RwLock<MetricsSnapshot>>,
    duration: Duration,
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

    let gpu_static_info =
        parse_gpu_static_info().expect("Critical: Failed to read static GPU hardware properties");

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
                    gpu: GPUInfo {
                      static_info: gpu_static_info.clone(),
                      dynamic_info: parse_gpu_dynamic_info().expect("Critical: Failed to read Dynamic GPU hardware properties"),
                    },
                    storage,
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
