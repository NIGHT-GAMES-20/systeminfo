import { ref, computed } from 'vue';
import type { MetricsSnapshot } from '../types/metrics';

const rawMetrics = ref<MetricsSnapshot | null>(null);
const isLoading = ref<boolean>(false);
const error = ref<Error | null>(null);

export function useSystemMetrics() {
  const updateMetrics = (data: MetricsSnapshot) => {
    rawMetrics.value = data;
  };

  const updatePollingRate = async (url: string = '/change-interval', sec: number) => {
    try{
      const res = await fetch(`${url}?seconds=${sec}`);
      if (!res.ok) throw new Error(`HTTP error! status: ${res.status}`);
      const data = await res.json()
      if (!data.ok) throw new Error(`Server error! Failed to Change Polling Rate: ${data.error}`)
    } catch (err) {
      error.value = err instanceof Error ? err : new Error(String(err));
    }
  };

  const fetchMetrics = async (url: string = '/metrics') => {
    isLoading.value = true;
    error.value = null;
    try {
      const res = await fetch(url);
      if (!res.ok) throw new Error(`HTTP error! status: ${res.status}`);
      const data: MetricsSnapshot = await res.json();
      rawMetrics.value = data;
    } catch (err) {
      error.value = err instanceof Error ? err : new Error(String(err));
    } finally {
      isLoading.value = false;
    }
  };

  // Separately accessible computed slices
  const hostname = computed(() => rawMetrics.value?.hostname ?? '');
  const cpu = computed(() => rawMetrics.value?.cpu ?? null);
  const ram_sticks = computed(() => rawMetrics.value?.ram_sticks ?? [])
  const memory = computed(() => rawMetrics.value?.memory ?? null);
  const swap = computed(() => rawMetrics.value?.swap ?? null);
  const gpu = computed(() => rawMetrics.value?.gpu ?? null);
  const gpuStatic = computed(() => rawMetrics.value?.gpu?.static_info ?? null);
  const gpuDynamic = computed(() => rawMetrics.value?.gpu?.dynamic_info ?? null);
  const storage = computed(() => rawMetrics.value?.storage ?? null)
  const temp_sensor_readings = computed(() => rawMetrics.value?.temp_sensor_readings ?? [])
  const network = computed(() => rawMetrics.value?.network_interfaces ?? []);
  const uptime = computed(() => rawMetrics.value?.uptime_secs ?? 0);
  const interval = computed(() => rawMetrics.value?.polling_interval_secs ?? 0);
  const timestamp = computed(() => rawMetrics.value?.timestamp ?? '');

  return {
    rawMetrics,
    isLoading,
    error,
    fetchMetrics,
    updateMetrics,
    updatePollingRate,
    hostname,
    cpu,
    ram_sticks,
    memory,
    swap,
    gpu,
    gpuStatic,
    gpuDynamic,
    storage,
    temp_sensor_readings,
    network,
    uptime,
    timestamp,
    interval
  };
}
