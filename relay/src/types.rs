use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
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
    pub storage: StorageSummary,
    pub temp_sensor_readings: Vec<TempSensorReading>,
    pub network_interfaces: NetworkInterfaces,
    pub processes: Vec<ProcessInfo>,
    pub uptime_secs: u64,
    pub polling_interval_secs: u64,
    pub timestamp: String,
}

#[derive(Serialize, Clone)]
pub struct CPUInfo {
    pub name: String,
    pub usage: Vec<f32>,
    pub avg_usage: f32,
    pub frequency: Vec<f64>,
    pub avg_frequency: f64,
    pub package_watts: f64,
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

#[derive(Serialize, Clone)]
pub struct RamStick {
    pub slot: u32,
    pub manufacturer: Option<String>,
    pub part_number: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SmartAttribute {
    pub id: u32,
    pub name: String,
    pub value: u32,
    pub worst: u32,
    pub thresh: u32,
    pub raw_value: u64,
    pub raw_string: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DriveSmartInfo {
    pub device: String,
    pub protocol: String, // "ATA", "NVMe", "SCSI"
    pub model_name: String,
    pub serial_number: String,
    pub firmware_version: String,
    pub capacity_bytes: u64,
    pub smart_passed: Option<bool>,
    pub temperature_celsius: Option<f64>,
    pub power_on_hours: Option<u64>,
    pub power_cycle_count: Option<u64>,
    /// NVMe wear level or SATA SSD wear indicator (0-100%)
    pub percentage_used: Option<u8>,
    /// Populated for SATA/ATA drives
    pub ata_attributes: Vec<SmartAttribute>,
    /// Populated if smartctl threw a warning or failed to open USB bridge
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MountUsageInfo {
    pub mount_point: String,
    pub device_name: String,
    pub file_system: String,
    pub total_gb: f64,
    pub used_gb: f64,
    pub available_gb: f64,
    pub usage_percent: f64,
    pub is_removable: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BlockDeviceIoStats {
    pub device: String, // e.g., "sda", "nvme0n1"
    pub read_mb_per_sec: f64,
    pub write_mb_per_sec: f64,
    pub read_iops: f64,
    pub write_iops: f64,
    pub total_read_gb: f64,
    pub total_written_gb: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StorageSummary {
    pub total_capacity_gb: f64,
    pub total_used_gb: f64,
    pub total_available_gb: f64,
    pub mounts: Vec<MountUsageInfo>,
    pub io_stats: Vec<BlockDeviceIoStats>,
    pub smart_drives: Vec<DriveSmartInfo>,
    pub smart_last_updated: Option<String>,
}

/// Internal state for calculating /proc/diskstats deltas
#[derive(Debug, Clone, Copy)]
pub struct DiskIoRawState {
    pub sectors_read: u64,
    pub sectors_written: u64,
    pub reads_completed: u64,
    pub writes_completed: u64,
    pub timestamp: std::time::Instant,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub user: String,
    pub command: String,
    pub cpu_usage: f32,
    pub memory_bytes: u64,
    pub memory_mb: f64,
    pub status: String,
    pub run_time_secs: u64,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub enum SortMetric {
    Cpu,
    Memory,
    Pid,
    RunTime,
}

#[derive(Serialize)]
pub struct JsonResponse {
    pub ok: bool,
    pub message: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SmartDeviceArgsPayload {
    /// Device path, e.g. "/dev/sdb" or "/dev/disk/by-id/usb-..."
    pub device: String,
    /// Extra smartctl arguments, e.g. ["-d", "sat", "-n", "standby"]
    pub args: Vec<String>,
}

#[derive(Deserialize)]
pub struct IntervalQuery {
    pub seconds: u64,
}

#[derive(Debug, Deserialize)]
pub struct ProcessesConfigPayload {
    pub count: usize,
    pub sort: SortMetric,
}

#[derive(Clone)]
pub struct AppState {
    pub metrics: Arc<RwLock<MetricsSnapshot>>,
    pub interval_secs: Arc<RwLock<u64>>,
    pub processes_config: Arc<RwLock<ProcessesConfig>>,
    pub smart_config: Arc<RwLock<SmartConfig>>,
    pub cancel_token: Arc<RwLock<CancellationToken>>,
}

#[derive(Clone, Copy)]
pub struct RaplState {
    pub energy_uj: u64,
    pub timestamp: std::time::Instant,
}

#[derive(Debug, Clone)]
pub struct SmartConfig {
    /// Target interval in seconds (enforced minimum: 300s / 5 mins)
    pub interval_secs: u64,
    /// Map of device path (e.g., "/dev/sda" or "/dev/disk/by-id/usb-...")
    /// to extra CLI arguments (e.g., vec!["-d".into(), "sat".into()])
    pub device_extra_args: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Copy)]
pub struct ProcessesConfig {
    pub count: usize,
    pub sort: SortMetric,
}
