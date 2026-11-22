<template>
  <div class="bg-mist-700/30 w-full min-h-16 py-2 px-5 flex flex-col justify-between backdrop-blur-sm text-white">
    <!-- Loaded State -->
    <div v-if="ClimateMetrics.isLoading" class="grid grid-cols-1 md:grid-cols-3 gap-4 items-center w-full h-full">

      <!-- Left: Weather Status & Metrics -->
      <div class="flex flex-col gap-1 justify-center">
        <div class="flex items-baseline gap-2">
          <span class="text-2xl font-bold">
            {{ ClimateMetrics.climateData.value?.current?.temperature_2m }}{{ ClimateMetrics.climateData.value?.units?.temperature_2m }}
          </span>
          <span class="text-sm font-medium text-gray-300">
            {{ ClimateMetrics.getSkyCondition(ClimateMetrics.climateData.value?.current?.weather_code) }}
          </span>
        </div>
        <div class="flex gap-3 text-xs text-gray-400">
          <span>Humidity: {{ ClimateMetrics.climateData.value?.current?.relative_humidity_2m }}{{ ClimateMetrics.climateData.value?.units?.relative_humidity_2m }}</span>
          <span>•</span>
          <span>Wind: {{ ClimateMetrics.climateData.value?.current?.wind_speed_10m }} {{ ClimateMetrics.climateData.value?.units?.wind_speed_10m }}, {{ClimateMetrics.getWindDirection(ClimateMetrics.climateData.value?.current?.wind_direction_10m)}}</span>
        </div>
      </div>

      <!-- Middle: Live Date & Time -->
      <div class="flex flex-col items-center justify-center text-center py-2 md:py-0">
        <span class="text-xl font-semibold tracking-wide">
          {{ currentTime }}
        </span>
        <span class="text-xs text-gray-400 uppercase tracking-wider">
          {{ currentDate }}
        </span>
      </div>

      <!-- Right: Location Display & Search Input -->
      <div class="flex flex-col items-end gap-2 justify-center min-w-0">
        <div class="flex items-center gap-1.5 text-xs text-gray-300 max-w-full min-w-0">
          <svg class="w-3.5 h-3.5 shrink-0 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17.657 16.657L13.414 20.9a1.998 1.998 0 01-2.827 0l-4.244-4.243a8 8 0 1111.314 0z" />
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 11a3 3 0 11-6 0 3 3 0 016 0z" />
          </svg>
          <span class="font-medium truncate" :title="fullAddress">{{ locationLabel }}</span>
        </div>
      </div>
    </div>

    <!-- Loading State -->
    <div v-else class="flex flex-1 items-center justify-center text-gray-400 text-sm">
      <div class="flex items-center gap-2">
        <div class="w-2 h-2 rounded-full bg-gray-400 animate-ping"></div>
        <span>Loading weather...</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onBeforeMount, onUnmounted, computed } from 'vue';
import { useClimateMetrics } from '../composables/useClimateMetrics';

const ClimateMetrics = useClimateMetrics()

const locationLabel = computed(() => {
  const raw = ClimateMetrics.climateData.value?.resolvedAddress
  const parts = Array.isArray(raw) ? raw : String(raw ?? '').split(',')

  const seen = new Set<string>()
  return parts
    .map(p => p.trim())
    .filter(p =>
      p &&
      /^[\x20-\x7E]+$/.test(p) &&
      !/^\d+$/.test(p) &&
      !/\bP\.?S\b|\(M\.Corp\.\)/i.test(p)
    )
    .filter(p => !seen.has(p.toLowerCase()) && seen.add(p.toLowerCase()))
    .join(', ')
})

// Optional: keep the full address for a tooltip
const fullAddress = computed(() => {
  const raw = ClimateMetrics.climateData.value?.resolvedAddress
  return Array.isArray(raw) ? raw.map(p => p.trim()).join(', ') : String(raw ?? '')
})

const currentTime = ref('');
const currentDate = ref('');
let clockTimer: ReturnType<typeof setInterval> | null = null;

function updateClock() {
  const now = new Date();
  currentTime.value = now.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
  currentDate.value = now.toLocaleDateString([], { weekday: 'short', month: 'short', day: 'numeric' });
}

onBeforeMount(async () => {
  updateClock();
  clockTimer = setInterval(updateClock, 1000);
});

onUnmounted(() => {
  if (clockTimer) clearInterval(clockTimer);
});


</script>
