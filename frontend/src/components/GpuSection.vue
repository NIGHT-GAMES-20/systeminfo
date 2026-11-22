<template>
  <div class="bg-mist-700/30 w-full min-w-0 h-auto py-5 px-10 flex flex-col justify-between backdrop-blur-sm text-white rounded-xl">
    <section class="flex flex-row items-start justify-between gap-4">
      <div class="flex flex-col gap-1">
        <span class="text-xl font-semibold text-amber-50">{{ gpuStatic?.product_name }}</span>
        <span class="text-sm text-gray-300">Driver Version {{ gpuStatic?.kmd_version }}</span>
      </div>
      <div class="grid grid-cols-2 grid-rows-2 justify-end gap-2 text-xs text-gray-300">
        <span class="rounded-full bg-white/5 px-3 py-1 font-mono tabular-nums"> cuda Version {{ gpuStatic?.umd_version }} </span>
        <span class="rounded-full bg-white/5 px-3 py-1 font-mono tabular-nums"> vbios Version {{ gpuStatic?.vbios_version }} </span>
        <span class="rounded-full bg-white/5 px-3 py-1 font-mono tabular-nums"> UUID {{ gpuStatic?.gpu_uuid }} </span>
        <span class="rounded-full bg-white/5 px-3 py-1 font-mono tabular-nums"> PDI {{ gpuStatic?.gpu_pdi }} </span>
      </div>
    </section>
    <section class="grid grid-cols-2 gap-3 mt-5">

      <div class="flex flex-row items-center rounded-lg border border-white/5 bg-black/20 p-3 justify-between">
        <div class="flex flex-row gap-3 items-center bg-amber-300/5 px-2 py-1 rounded">
          <Fan :size="20" class="text-amber-200"/>
          <span class="text-amber-200">Fan</span>
        </div>
        <span class="text-amber-300"> {{ gpuDynamic?.fan_speed }}%</span>
      </div>

      <div class="flex flex-row items-center rounded-lg border border-white/5 bg-black/20 p-3 justify-between">
        <div class="flex flex-row gap-3 items-center bg-sky-300/5 px-2 py-1 rounded">
          <Gauge :size="20" class="text-sky-200"/>
          <span class="text-sky-200">Performance State</span>
        </div>
        <span class="text-sky-300"> {{ gpuDynamic?.performance_state }}</span>
      </div>
    </section>
    <section class="mt-6 grid grid-cols-1 gap-4 lg:grid-cols-2 min-[1900px]:grid-cols-4">
      <div class="flex flex-col gap-3 rounded-lg border border-white/5 bg-black/20 p-4">
        <LineChart
          title="Temperature"
          :latest-value="Number((gpuDynamic?.current_temp ?? 0).toFixed(1))"
          unit="&deg;C"
          class="h-40 w-full mb-10"
          titleClass="text-lg text-orange-400"
          subtextClass="hidden"
          valueClass="text-lg text-orange-400 font-mono font-bold"
          strokeClass="text-orange-500"
        />
      </div>
      <div class="flex flex-col gap-3 rounded-lg border border-white/5 bg-black/20 p-4">
        <LineChart
          title="Power Usage"
          :latest-value="Number((gpuDynamic?.power_usage ?? 0).toFixed(1))"
          :maxScale="gpuStatic?.power_info?.max_limit"
          unit=" W"
          class="h-40 w-full mb-10"
          titleClass="text-lg text-rose-300"
          subtextClass="hidden"
          valueClass="text-lg text-rose-300 font-mono font-bold"
          strokeClass="text-rose-400"
        />
        <div class="grid grid-cols-2 gap-2 text-xs text-gray-400">
          <span>Max Limit: <span class="font-mono text-gray-200">{{ gpuStatic?.power_info?.max_limit }} W</span></span>
        </div>
      </div>
    </section>
    <section class="mt-6 flex flex-col gap-4">
      <div class="flex flex-col justify-between">
        <div class="flex flex-row items-center gap-2 text-sky-400">
          <Cpu :size="20" />
          <h2 class="text-lg font-semibold ">Utilization</h2>
        </div>
        <div class="mt-2 grid grid-cols-1 gap-4 lg:grid-cols-2 min-[1900px]:grid-cols-4">
          <div class="flex flex-col gap-3 rounded-lg border border-white/5 bg-black/20 p-4">
            <LineChart
              title="GPU"
              :latest-value="Number((gpuDynamic?.utilization?.gpu ?? 0).toFixed(1))"
              unit="%"
              :maxScale="100"
              class="h-40 w-full mb-10"
              titleClass="text-lg text-sky-200"
              subtextClass="hidden"
              valueClass="text-lg text-sky-200 font-mono font-bold"
              strokeClass="text-sky-400"
            />
            <Bars :values="under_util_gpu" :labels="['Encoder', 'Decoder']" :max="100" unit="%" bar-class="bg-sky-400" />
          </div>
          <div class="flex flex-col gap-3 rounded-lg border border-white/5 bg-black/20 p-4">
            <LineChart
              title="VRAM"
              :latest-value="Number((gpuDynamic?.utilization?.memory ?? 0).toFixed(1))"
              unit="%"
              :maxScale="100"
              class="h-40 w-full mb-10"
              titleClass="text-lg text-sky-200"
              subtextClass="hidden"
              valueClass="text-lg text-sky-200 font-mono font-bold"
              strokeClass="text-sky-400"
            />
            <Bars :values="under_util_vram" :labels="['JPEG', 'OFA']" :max="100" unit="%" bar-class="bg-sky-400" />
          </div>
        </div>
      </div>
    </section>
    <section class="mt-6 flex flex-col gap-4">
      <div class="flex flex-col justify-between">
        <div class="flex flex-row items-center gap-2 text-emerald-400">
          <MemoryStick :size="20" />
          <h2 class="text-lg font-semibold ">Memory</h2>
        </div>
        <div class="mt-2 grid grid-cols-1 gap-4 lg:grid-cols-2 min-[1900px]:grid-cols-4">
          <div class="flex flex-col gap-3 rounded-lg border border-white/5 bg-black/20 p-4">
            <LineChart
              title="FB Memory Usage"
              :latest-value="Number((gpuDynamic?.fb_memory?.used_gb ?? 0).toFixed(3))"
              :maxScale="Number((gpuDynamic?.fb_memory?.total_gb ?? 0).toFixed(2))"
              unit=" GB"
              class="h-40 w-full mb-10"
              titleClass="text-lg text-emerald-400"
              subtextClass="hidden"
              valueClass="text-lg text-emerald-400 font-mono font-bold"
              strokeClass="text-emerald-400"
            />
            <Bars :values="under_fb_mem" :labels="['Free', 'Available']" :max="Number((gpuDynamic?.fb_memory?.total_gb ?? 0).toFixed(2))" :decimals="2" unit=" GB" bar-class="bg-emerald-400" />
          </div>
          <div class="flex flex-col gap-3 rounded-lg border border-white/5 bg-black/20 p-4">
            <LineChart
              title="BAR1 Memory Usage"
              :latest-value="Number((gpuDynamic?.bar1_memory?.used_gb ?? 0).toFixed(4))"
              :maxScale="Number((gpuDynamic?.bar1_memory?.total_gb ?? 0).toFixed(3))"
              unit=" GB"
              class="h-40 w-full mb-10"
              titleClass="text-lg text-emerald-400"
              subtextClass="hidden"
              valueClass="text-lg text-emerald-400 font-mono font-bold"
              strokeClass="text-emerald-400"
            />
            <Bars class="max-w-[50%]" :values="under_bar1_mem" :labels="['Free']" :max="Number((gpuDynamic?.bar1_memory?.total_gb ?? 0).toFixed(4))" :decimals="4" unit=" GB" bar-class="bg-emerald-400" />
          </div>
        </div>
      </div>
    </section>
    <section class="mt-6 flex flex-col gap-4">
      <div class="flex flex-col justify-between">
        <div class="flex flex-row items-center gap-2 text-amber-400">
          <Clock :size="20" />
          <h2 class="text-lg font-semibold ">Clock</h2>
        </div>
        <div class="mt-2 grid grid-cols-2 gap-4 lg:grid-cols-4 min-[1900px]:grid-cols-8">
          <div class="flex flex-col gap-3 rounded-lg border border-white/5 bg-black/20 p-4">
            <LineChart
              title="Graphics"
              :latest-value="Number((gpuDynamic?.clock.graphics ?? 0).toFixed(3))"
              :maxScale="Number((gpuStatic?.clock_info.max_graphics ?? 0).toFixed(2))"
              unit=" Mhz"
              class="h-40 w-full mb-10"
              titleClass="text-lg text-amber-400"
              subtextClass="hidden"
              valueClass="text-lg text-amber-400 font-mono font-bold"
              strokeClass="text-amber-400"
            />
          </div>
          <div class="flex flex-col gap-3 rounded-lg border border-white/5 bg-black/20 p-4">
            <LineChart
              title="SM"
              :latest-value="Number((gpuDynamic?.clock.sm ?? 0).toFixed(4))"
              :maxScale="Number((gpuStatic?.clock_info.max_sm ?? 0).toFixed(3))"
              unit=" Mhz"
              class="h-40 w-full mb-10"
              titleClass="text-lg text-amber-400"
              subtextClass="hidden"
              valueClass="text-lg text-amber-400 font-mono font-bold"
              strokeClass="text-amber-400"
            />
          </div>
          <div class="flex flex-col gap-3 rounded-lg border border-white/5 bg-black/20 p-4">
            <LineChart
              title="Memory"
              :latest-value="Number((gpuDynamic?.clock.memory ?? 0).toFixed(3))"
              :maxScale="Number((gpuStatic?.clock_info.max_memory ?? 0).toFixed(2))"
              unit=" Mhz"
              class="h-40 w-full mb-10"
              titleClass="text-lg text-amber-400"
              subtextClass="hidden"
              valueClass="text-lg text-amber-400 font-mono font-bold"
              strokeClass="text-amber-400"
            />
          </div>
          <div class="flex flex-col gap-3 rounded-lg border border-white/5 bg-black/20 p-4">
            <LineChart
              title="Video"
              :latest-value="Number((gpuDynamic?.clock.video ?? 0).toFixed(3))"
              :maxScale="Number((gpuStatic?.clock_info.max_video ?? 0).toFixed(2))"
              unit=" Mhz"
              class="h-40 w-full mb-10"
              titleClass="text-lg text-amber-400"
              subtextClass="hidden"
              valueClass="text-lg text-amber-400 font-mono font-bold"
              strokeClass="text-amber-400"
            />
          </div>
        </div>
      </div>
    </section>
    <section class="mt-6 flex flex-col gap-4">
      <div class="flex flex-col justify-between">
        <div class="flex flex-row items-center gap-2 text-indigo-400">
          <ChevronsLeftRightEllipsis :size="20" />
          <h2 class="text-lg font-semibold ">PCIe</h2>
        </div>
        <div class="grid grid-cols-2 gap-3 pt-5 sm:grid-cols-3">
          <div
            v-for="(data, index) in gpuStatic?.pcie_info"
            :key="index"
            class="flex flex-col gap-3 rounded-lg border border-white/5 bg-black/20 p-4"
          >
            <div class="flex items-center justify-between gap-2">
              <div class="flex items-center gap-2">
                <span class="text-sm rounded bg-indigo-400/15 font-mono items-center justify-center text-indigo-300 h-auto w-auto px-2">{{ index }}</span>
              </div>
              <span class="px-2 py-0.5 text-xs text-gray-300">{{ data }}</span>
            </div>
          </div>
        </div>
        <div class="mt-2 grid grid-cols-1 gap-4 lg:grid-cols-2 min-[1900px]:grid-cols-4">
          <div
            v-for="(data, index) in gpuDynamic?.pcie_throughput"
            :key="index"
          >
            <div class="flex flex-col gap-3 rounded-lg border border-white/5 bg-black/20 p-4">
              <LineChart
                :title="index"
                :latest-value="Number((data / 1024).toFixed(3))"
                unit=" MB/s"
                class="h-40 w-full mb-10"
                titleClass="text-lg text-indigo-300"
                subtextClass="hidden"
                valueClass="text-lg text-indigo-300 font-mono font-bold"
                strokeClass="text-indigo-300"
              />
            </div>
          </div>
        </div>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { useSystemMetrics } from '../composables/useSystemMetrics';
import { Fan, Gauge, MemoryStick, Cpu, Clock, ChevronsLeftRightEllipsis } from '@lucide/vue';
import LineChart from './LineChart.vue';
import Bars from './Bars.vue';

const {
  gpuStatic,
  gpuDynamic,
} = useSystemMetrics()

const under_util_gpu = computed(() => [
  gpuDynamic.value?.utilization.encoder ?? 0,
  gpuDynamic.value?.utilization.decoder ?? 0
])

const under_util_vram = computed(() => [
  gpuDynamic.value?.utilization.jpeg ?? 0,
  gpuDynamic.value?.utilization.ofa ?? 0
])

const under_fb_mem = computed(() => [
  gpuDynamic.value?.fb_memory.free_gb ?? 0,
  gpuDynamic.value?.fb_memory.reserved_gb ?? 0,
])

const under_bar1_mem = computed(() => [
  gpuDynamic.value?.bar1_memory.free_gb ?? 0,
])
</script>
