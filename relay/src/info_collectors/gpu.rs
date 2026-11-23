use crate::types::*;
use regex::Regex;
use std::process::Command;

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
