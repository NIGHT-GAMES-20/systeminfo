export interface CPUInfo {
  name: string;
  usage: number[];
  avg_usage: number;
  frequency: number[];
  avg_frequency: number;
  package_watts: number;
}

export interface MemoryInfo {
  total_gb: number;
  used_gb: number;
  free_gb: number;
  available_gb: number;
}

export interface SwapInfo {
  total_gb: number;
  used_gb: number;
  free_gb: number;
}

export interface NetworkInterfaceInfo {
  name: string;
  tx_bytes: string;
  rx_bytes: string;
  speed_mbps: string;
}

export type NetworkInterfaces = NetworkInterfaceInfo[];

export interface GPUPcieStaticInfo {
  bus: string;
  device_id: string;
  bus_id: string;
  current_pcie_gen: string;
  current_link_width: string;
}

export interface GPUPowerStaticInfo {
  min_limit: number;
  max_limit: number;
}

export interface GPUClockStaticInfo {
  max_graphics: number;
  max_sm: number;
  max_memory: number;
  max_video: number;
}

export interface GPUStaticInfo {
  product_name: string;
  kmd_version: string;
  umd_version: string;
  vbios_version: string;
  gpu_uuid: string;
  gpu_pdi: string;
  pcie_info: GPUPcieStaticInfo;
  power_info: GPUPowerStaticInfo;
  clock_info: GPUClockStaticInfo;
}

export interface FbGPUMemory {
  total_gb: number;
  used_gb: number;
  free_gb: number;
  reserved_gb: number;
}

export interface BarGPUMemory {
  total_gb: number;
  used_gb: number;
  free_gb: number;
}

export interface GPUUtilization {
  gpu: number;
  memory: number;
  encoder: number;
  decoder: number;
  jpeg: number;
  ofa: number;
}

export interface GPUClock {
  graphics: number;
  sm: number;
  memory: number;
  video: number;
}

export interface GPUPcieDynamicInfo {
  tx_throughput: number;
  rx_throughput: number;
}

export interface GPUDynamicInfo {
  fan_speed: number;
  performance_state: string;
  current_temp: number;
  fb_memory: FbGPUMemory;
  bar1_memory: BarGPUMemory;
  utilization: GPUUtilization;
  clock: GPUClock;
  power_usage: number;
  pcie_throughput: GPUPcieDynamicInfo;
}

export interface GPUInfo {
  static_info: GPUStaticInfo;
  dynamic_info: GPUDynamicInfo;
}

export interface TempSensorReading {
  id: string;
  chip: string;
  sensor: string;
  raw_label: string;
  temp_celsius: number | undefined;
  max_celsius: number | undefined;
  crit_celsius: number | undefined;
}

 export interface RamStick {
    slot: number,
    manufacturer: string,
    part_number: string,
}

export interface SmartAttribute {
  id: number;
  name: string;
  value: number;
  worst: number;
  thresh: number;
  raw_value: number;
  raw_string: string;
}

export interface DriveSmartInfo {
  device: string;
  protocol: "ATA" | "NVMe" | "SCSI" | "Unknown" | (string & {});
  model_name: string;
  serial_number: string;
  firmware_version: string;
  capacity_bytes: number;
  smart_passed: boolean | null;
  temperature_celsius: number | null;
  power_on_hours: number | null;
  power_cycle_count: number | null;
  /** NVMe wear level or SATA SSD wear indicator (0-100%) */
  percentage_used: number | null;
  /** Populated for SATA/ATA drives */
  ata_attributes: SmartAttribute[];
  /** Populated if smartctl threw a warning or failed to open USB bridge */
  error: string | null;
}

export interface MountUsageInfo {
  mount_point: string;
  device_name: string;
  file_system: string;
  total_gb: number;
  used_gb: number;
  available_gb: number;
  usage_percent: number;
  is_removable: boolean;
}

export interface BlockDeviceIoStats {
  /** e.g., "sda", "nvme0n1" */
  device: string;
  read_mb_per_sec: number;
  write_mb_per_sec: number;
  read_iops: number;
  write_iops: number;
  total_read_gb: number;
  total_written_gb: number;
}

export interface StorageSummary {
  total_capacity_gb: number;
  total_used_gb: number;
  total_available_gb: number;
  mounts: MountUsageInfo[];
  io_stats: BlockDeviceIoStats[];
  smart_drives: DriveSmartInfo[];
  /** ISO-8601 / RFC-3339 timestamp string, or null before first SMART scan */
  smart_last_updated: string | null;
}

export interface MetricsSnapshot {
  hostname: string;
  cpu: CPUInfo;
  ram_sticks: RamStick[];
  memory: MemoryInfo;
  swap: SwapInfo;
  gpu: GPUInfo;
  storage: StorageSummary;
  temp_sensor_readings: TempSensorReading[];
  network_interfaces: NetworkInterfaces;
  uptime_secs: number;
  polling_interval_secs: number;
  timestamp: string;
}
