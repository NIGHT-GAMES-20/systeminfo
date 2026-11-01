use crate::types::*;

impl Default for MetricsSnapshot {
    fn default() -> Self {
        MetricsSnapshot {
            hostname: "initializing".to_string(),
            cpu: CPUInfo {
                name: "initializing".to_string(),
                usage: vec![],
                avg_usage: 0.0,
                frequency: vec![],
                avg_frequency: 0.0,
            },
            ram_sticks: vec![],
            memory: MemoryInfo {
                total_gb: 0.0,
                used_gb: 0.0,
                free_gb: 0.0,
                available_gb: 0.0,
            },
            swap: SwapInfo {
                total_gb: 0.0,
                used_gb: 0.0,
                free_gb: 0.0,
            },
            gpu: GPUInfo {
                static_info: GPUStaticInfo::default(),
                dynamic_info: GPUDynamicInfo::default(),
            },
            temp_sensor_readings: vec![],
            network_interfaces: vec![],
            uptime_secs: 0,
            polling_interval_secs: 5,
            timestamp: "".to_string(),
        }
    }
}

impl Default for GPUStaticInfo {
    fn default() -> Self {
        GPUStaticInfo {
            product_name: "".to_string(),
            kmd_version: "".to_string(),
            umd_version: "".to_string(),
            vbios_version: "".to_string(),
            gpu_uuid: "".to_string(),
            gpu_pdi: "".to_string(),
            pcie_info: GPUPcieStaticInfo {
                bus: "".to_string(),
                device_id: "".to_string(),
                bus_id: "".to_string(),
                current_pcie_gen: "".to_string(),
                current_link_width: "".to_string(),
            },
            power_info: GPUPowerStaticInfo {
                min_limit: 0.0,
                max_limit: 0.0,
            },
            clock_info: GPUClockStaticInfo {
                max_graphics: 0,
                max_sm: 0,
                max_memory: 0,
                max_video: 0,
            },
        }
    }
}

impl Default for GPUDynamicInfo {
    fn default() -> Self {
        GPUDynamicInfo {
            fan_speed: 0,
            performance_state: "".to_string(),
            fb_memory: FbGPUMemory {
                total_gb: 0.0,
                used_gb: 0.0,
                free_gb: 0.0,
                reserved_gb: 0.0,
            },
            bar1_memory: BarGPUMemory {
                total_gb: 0.0,
                used_gb: 0.0,
                free_gb: 0.0,
            },
            current_temp: 0.0,
            utilization: GPUUtilization {
                gpu: 0.0,
                memory: 0.0,
                encoder: 0.0,
                decoder: 0.0,
                jpeg: 0.0,
                ofa: 0.0,
            },
            clock: GPUClock {
                graphics: 0.0,
                sm: 0.0,
                memory: 0.0,
                video: 0.0,
            },
            power_usage: 0.0,
            pcie_throughput: GPUPcieDynamicInfo {
                tx_throughput: 0.0,
                rx_throughput: 0.0,
            },
        }
    }
}
