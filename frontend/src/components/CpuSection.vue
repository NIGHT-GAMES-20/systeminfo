<template>
  <div class="w-full rounded-xl bg-mist-700/30 p-5 text-white backdrop-blur-sm md:px-8">
    <!-- Header -->
    <div class="flex flex-row items-start justify-between gap-4">
      <div class="flex flex-col gap-1">
        <span class="text-xl font-semibold text-amber-50">{{ hostname }}</span>
        <span class="text-sm text-gray-300">{{ cpu?.name }}</span>
      </div>
      <div class="flex flex-wrap justify-end gap-2 text-xs text-gray-300">
        <span class="rounded-full bg-white/5 px-3 py-1 font-mono tabular-nums">
          Up {{ secondsToHoursMinutesSeconds(uptime) }}
        </span>
        <span class="rounded-full bg-white/5 px-3 py-1 font-mono">Poll {{ interval }}s</span>
        <span class="hidden rounded-full bg-white/5 px-3 py-1 font-mono xl:inline">{{ timestamp }}</span>
      </div>
    </div>

    <!-- Panels: 1 col on mobile, 2 on desktop, 4 on very wide screens -->
    <div class="mt-6 grid grid-cols-1 gap-4 lg:grid-cols-2 min-[1900px]:grid-cols-4">
      <!-- Usage -->
      <section class="flex flex-col gap-3 rounded-lg border border-white/5 bg-black/20 p-4">
        <LineChart
          title="Usage"
          :latest-value="Number((cpu?.avg_usage ?? 0).toFixed(1))"
          :maxScale="100"
          class="h-40 w-full mb-10"
          titleClass="text-lg text-sky-200"
          subtextClass="hidden"
          valueClass="text-lg text-sky-200 font-mono font-bold"
        />
        <Bars :values="cpu?.usage ?? []" :max="100" unit="%" bar-class="bg-sky-400" />
      </section>

      <!-- Frequency -->
      <section class="flex flex-col gap-3 rounded-lg border border-white/5 bg-black/20 p-4">
        <LineChart
          title="Frequency"
          :latest-value="Number((cpu?.avg_frequency ?? 0).toFixed(2))"
          unit="Ghz"
          class="h-40 w-full mb-10"
          titleClass="text-lg text-amber-200"
          subtextClass="hidden"
          valueClass="text-lg text-amber-200 font-mono font-bold"
          strokeClass="text-amber-400"
        />
        <Bars :values="cpu?.frequency ?? []" :max="5" unit="GHz" :decimals="2" bar-class="bg-amber-400" />
      </section>

      <!-- Temperature -->
      <section class="flex flex-col gap-3 rounded-lg border border-white/5 bg-black/20 p-4">
        <LineChart
          title="Temperature"
          :latest-value="cpu_readings[0]?.temp_celsius ?? 0"
          :maxScale="80"
          :minScale="20"
          unit="&deg;C"
          class="h-40 w-full mb-10"
          titleClass="text-lg text-orange-400"
          subtextClass="hidden"
          valueClass="text-lg text-orange-400 font-mono font-bold"
          strokeClass="text-orange-500"
        />
        <div class="grid grid-cols-2 gap-2 text-xs text-gray-400">
          <span>Sensor: <span class="font-mono text-gray-200">{{ cpu_readings[0]?.chip }}</span></span>
          <span>Max: <span class="font-mono text-gray-200">{{ cpu_readings[0]?.max_celsius }}°C</span></span>
        </div>
      </section>

      <!-- Power -->
      <section class="flex flex-col gap-3 rounded-lg border border-white/5 bg-black/20 p-4">
        <LineChart
          title="Power Usage"
          :latest-value="Number((cpu?.package_watts ?? 0).toFixed(1))"
          :maxScale="35"
          :minScale="0"
          unit="W"
          class="h-40 w-full mb-10"
          titleClass="text-lg text-rose-300"
          subtextClass="hidden"
          valueClass="text-lg text-rose-300 font-mono font-bold"
          strokeClass="text-rose-400"
        />
      </section>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useSystemMetrics } from '../composables/useSystemMetrics';
import Bars from "./Bars.vue";
import LineChart from './LineChart.vue';
import type { TempSensorReading } from '../types/metrics';

const {
  hostname,
  cpu,
  uptime,
  timestamp,
  temp_sensor_readings,
  interval
} = useSystemMetrics()

const cpu_readings = computed(() => {
  return temp_sensor_readings.value.filter(
    (reading: TempSensorReading) => reading.chip === 'k10temp'
  )
})

function secondsToHoursMinutesSeconds(totalSeconds: number): string {
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = Math.floor(totalSeconds % 60);

  const pad = (num: number): string => num.toString().padStart(2, '0');

  return `${pad(hours)}:${pad(minutes)}:${pad(seconds)}`;
}
</script>
