use crate::types::*;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::process::Command;
use std::str;
use sysinfo::Disks;

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
