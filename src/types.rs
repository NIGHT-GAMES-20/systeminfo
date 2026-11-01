use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

#[derive(Serialize, Clone)]
pub struct MetricsSnapshot {
    pub hostname: String,
    pub cpu: CPUInfo,
    pub ram_sticks: Vec<RamStick>,
    pub memory: MemoryInfo,
    pub swap: SwapInfo,
    pub gpu: GPUInfo,
    pub temp_sensor_readings: Vec<TempSensorReading>,
    pub network_interfaces: NetworkInterfaces,
    pub uptime_secs: u64,
    pub polling_interval_secs: u8,
    pub timestamp: String,
}

#[derive(Serialize, Clone)]
pub struct CPUInfo {
    pub name: String,
    pub usage: Vec<f32>,
    pub avg_usage: f32,
    pub frequency: Vec<f64>,
    pub avg_frequency: f64,
}

#[derive(Serialize, Clone)]
pub struct MemoryInfo {
    pub total_gb: f64,
    pub used_gb: f64,
    pub free_gb: f64,
    pub available_gb: f64,
}

#[derive(Serialize, Clone)]
pub struct SwapInfo {
    pub total_gb: f64,
    pub used_gb: f64,
    pub free_gb: f64,
}

#[derive(Serialize, Clone)]
pub struct NetworkInterfaceInfo {
    pub name: String,
    pub tx_bytes: String,
    pub rx_bytes: String,
    pub speed_mbps: String,
}

pub type NetworkInterfaces = Vec<NetworkInterfaceInfo>;

#[derive(Serialize, Clone)]
pub struct GPUInfo {
    pub static_info: GPUStaticInfo,
    pub dynamic_info: GPUDynamicInfo,
}

#[derive(Serialize, Clone)]
pub struct GPUStaticInfo {
    pub product_name: String,
    pub kmd_version: String,
    pub umd_version: String,
    pub vbios_version: String,
    pub gpu_uuid: String,
    pub gpu_pdi: String,
    pub pcie_info: GPUPcieStaticInfo,
    pub power_info: GPUPowerStaticInfo,
    pub clock_info: GPUClockStaticInfo,
}

#[derive(Serialize, Clone)]
pub struct GPUPcieStaticInfo {
    pub bus: String,
    pub device_id: String,
    pub bus_id: String,
    pub current_pcie_gen: String,
    pub current_link_width: String,
}

#[derive(Serialize, Clone)]
pub struct GPUPowerStaticInfo {
    pub min_limit: f64,
    pub max_limit: f64,
}

#[derive(Serialize, Clone)]
pub struct GPUClockStaticInfo {
    pub max_graphics: u32,
    pub max_sm: u32,
    pub max_memory: u32,
    pub max_video: u32,
}

#[derive(Serialize, Clone)]
pub struct GPUDynamicInfo {
    pub fan_speed: u32,
    pub performance_state: String,
    pub current_temp: f64,
    pub fb_memory: FbGPUMemory,
    pub bar1_memory: BarGPUMemory,
    pub utilization: GPUUtilization,
    pub clock: GPUClock,
    pub power_usage: f64,
    pub pcie_throughput: GPUPcieDynamicInfo,
}

#[derive(Serialize, Clone)]
pub struct FbGPUMemory {
    pub total_gb: f64,
    pub used_gb: f64,
    pub free_gb: f64,
    pub reserved_gb: f64,
}

#[derive(Serialize, Clone)]
pub struct BarGPUMemory {
    pub total_gb: f64,
    pub used_gb: f64,
    pub free_gb: f64,
}

#[derive(Serialize, Clone)]
pub struct GPUUtilization {
    pub gpu: f64,
    pub memory: f64,
    pub encoder: f64,
    pub decoder: f64,
    pub jpeg: f64,
    pub ofa: f64,
}

#[derive(Serialize, Clone)]
pub struct GPUClock {
    pub graphics: f64,
    pub sm: f64,
    pub memory: f64,
    pub video: f64,
}

#[derive(Serialize, Clone)]
pub struct GPUPcieDynamicInfo {
    pub tx_throughput: f64,
    pub rx_throughput: f64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TempSensorReading {
    pub id: String,                // e.g., "hwmon2_1"
    pub chip: String,              // e.g., "k10temp" or "gigabyte_wmi"
    pub sensor: String,            // e.g., "Tctl" or "temp1"
    pub raw_label: String,         // e.g., "k10temp Tctl"
    pub temp_celsius: Option<f32>, // e.g., Some(58.0)
    pub max_celsius: Option<f32>,  // e.g., Some(70.0)
    pub crit_celsius: Option<f32>, // e.g., Some(70.0)
}

#[derive(Serialize)]
pub struct JsonResponse {
    pub ok: bool,
    pub message: Option<String>,
    pub error: Option<String>,
}

#[derive(Deserialize)]
pub struct IntervalQuery {
    pub seconds: u64,
}

pub struct AppState {
    pub metrics: Arc<RwLock<MetricsSnapshot>>,
    pub interval_secs: Arc<RwLock<u64>>,
    pub cancel_token: Arc<RwLock<CancellationToken>>,
}

#[derive(Serialize, Clone)]
pub struct RamStick {
    pub slot: u32,
    pub manufacturer: Option<String>,
    pub part_number: Option<String>,
}
