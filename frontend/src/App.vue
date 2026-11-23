<template>
  <div class="flex flex-row min-h-screen">
    <SideBar @selected="changeSelection" :selected="selected" />
    <div class="flex flex-1 flex-col w-full min-w-0">
      <ClimateSection/>
      <div class="flex flex-1 flex-col gap-5 pt-10 items-center w-full">
        <SectionWrapper class="max-w-[calc(100%-8rem)]" :id="'cpu'" :activeSection="selected" @section-enter="changeSelection">
          <CpuSection />
        </SectionWrapper>
        <SectionWrapper class="max-w-[calc(100%-8rem)]" :id="'ram'" :activeSection="selected" @section-enter="changeSelection">
          <MemorySection />
        </SectionWrapper>
        <SectionWrapper class="max-w-[calc(100%-8rem)]" :id="'gpu'" :activeSection="selected" @section-enter="changeSelection">
          <GpuSection />
        </SectionWrapper>
      </div>
    </div>
  </div>
</template>


<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import SideBar from './components/SideBar.vue'
import ClimateSection from './components/ClimateSection.vue';
import SectionWrapper from './components/SectionWrapper.vue';
import CpuSection from './components/CpuSection.vue';
import MemorySection from './components/MemorySection.vue';
import GpuSection from './components/GpuSection.vue';
import { useSystemMetrics } from './composables/useSystemMetrics';
import { useClimateMetrics } from './composables/useClimateMetrics';

const selected = ref("cpu")
const DEFAULT_RATE = 5 // seconds

function changeSelection(id: string): void {
  selected.value = id;
  console.log(id)
}

// ?rate=1 in the URL > localStorage > default
function resolvePollingRate(): number {
  const fromUrl = new URLSearchParams(location.search).get('rate')
  const fromStorage = localStorage.getItem('polling_rate')
  const n = Number(fromUrl ?? fromStorage)
  return Number.isFinite(n) && n > 0 ? n : DEFAULT_RATE
}

let metricsTimer: number | undefined
let updatePollingRateTimer: number | undefined
let climateTimer: number | undefined

onMounted(async () => {
  const systemMetrics = useSystemMetrics()
  const climateMetrics = useClimateMetrics()
  const rate = resolvePollingRate()
  localStorage.setItem('polling_rate', String(rate))

  // Always tell the backend, so both sides agree even with an empty profile
  await systemMetrics.updatePollingRate('/systeminfo/api/set-interval', rate)
  await systemMetrics.fetchMetrics('/systeminfo/api/metrics')
  await climateMetrics.updateClimateMetrics()

  metricsTimer = window.setInterval(async () => {
    await systemMetrics.fetchMetrics('/systeminfo/api/metrics')
  }, rate * 1000)

  updatePollingRateTimer = window.setInterval(async () => {
    await systemMetrics.updatePollingRate('/systeminfo/api/set-interval', rate)
  }, 60 * 1000)

  climateTimer = window.setInterval(() => {
    climateMetrics.updateClimateMetrics()
  }, 10 * 60 * 1000)
})

onUnmounted(() => {
  clearInterval(metricsTimer)
  clearInterval(climateTimer)
  clearInterval(updatePollingRateTimer)
})

</script>
