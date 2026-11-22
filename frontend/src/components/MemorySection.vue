<template>
  <div class="bg-mist-700/30 w-full min-w-0 h-auto py-5 px-10 flex flex-col justify-between backdrop-blur-sm text-white rounded-xl">
    <span class="text-xl font-semibold text-amber-50"> Memory </span>
    <div class="grid grid-cols-1 gap-3 pt-5 sm:grid-cols-2">
      <div
        v-for="(data, index) in ram_sticks"
        :key="index"
        class="flex flex-col gap-3 rounded-lg border border-white/5 bg-black/20 p-4"
      >
        <!-- Header: slot badge + manufacturer -->
        <div class="flex items-center justify-between gap-2">
          <div class="flex items-center gap-2">
            <span class="text-sm rounded bg-sky-400/15 font-mono items-center justify-center text-sky-300 h-auto w-auto px-2">Slot {{ data.slot }}</span>
          </div>
          <span class="truncate rounded-full bg-white/5 px-2 py-0.5 text-xs text-gray-300"/>
        </div>

        <!-- Details -->
        <dl class="grid grid-cols-[auto_1fr] items-baseline gap-x-3 gap-y-1 text-xs">
          <dt class="text-gray-500">Part No. </dt>
          <dd class="truncate font-mono text-gray-200" :title="data.part_number">
            {{ data.part_number }}
          </dd>

          <dt class="text-gray-500">Manufacturer </dt>
          <dd class="truncate text-gray-200" :title="data.manufacturer">
            {{ data.manufacturer }}
          </dd>
        </dl>
      </div>
    </div>
    <div class="grid grid-cols-2 gap-3 mt-10">
      <section class="flex flex-col gap-3 rounded-lg border border-white/5 bg-black/20 p-4">
        <LineChart
          title="RAM Usage"
          :latest-value="Number((memory?.used_gb ?? 0).toFixed(3))"
          :maxScale="Number((memory?.total_gb ?? 0).toFixed(1))"
          unit="GB"
          class="pt-5 w-full"
          titleClass="text-l text-emerald-400"
          subtextClass="hidden"
          valueClass="text-l text-emerald-400 font-mono font-bold"
          strokeClass="text-emerald-400"
        />
        <Bars :values="mem_etc" :labels="['Free', 'Available']" :max="memory?.total_gb ?? 1" unit=" GB" :decimals="1" bar-class="bg-emerald-400" />
      </section>
      <section class="flex flex-col gap-3 rounded-lg border border-white/5 bg-black/20 p-4">
        <LineChart
          title="Swap Usage"
          :latest-value="Number((swap?.used_gb ?? 0).toFixed(3))"
          :maxScale="Number((swap?.total_gb ?? 0).toFixed(1))"
          unit="GB"
          class="pt-5 w-full"
          titleClass="text-l text-emerald-200"
          subtextClass="hidden"
          valueClass="text-l text-emerald-200 font-mono font-bold"
          strokeClass="text-emerald-200"
        />
        <Bars :values="swap_etc" :labels="['Free']" :max="swap?.total_gb ?? 1" unit=" GB" :decimals="1" bar-class="bg-emerald-200" class="max-w-[50%]"/>
      </section>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue"
import { useSystemMetrics } from '../composables/useSystemMetrics';
import LineChart from './LineChart.vue';
import Bars from './Bars.vue';

const {
  ram_sticks,
  memory,
  swap,
} = useSystemMetrics()

const mem_etc = computed(() => [
  memory.value?.free_gb ?? 0,
  memory.value?.available_gb ?? 0,
])

const swap_etc = computed(() => [
  swap.value?.free_gb ?? 0,
])


</script>
