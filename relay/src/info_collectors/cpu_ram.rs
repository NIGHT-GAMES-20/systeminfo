use crate::helper_functions::*;
use crate::types::*;
use std::collections::BTreeMap;
use std::fs;
use std::process::Command;
use sysinfo::Components;

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
