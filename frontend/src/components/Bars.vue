<template>
  <div class="grid gap-x-4 gap-y-2" :style="{ gridTemplateColumns: `repeat(${cols}, minmax(0, 1fr))` }">
    <div v-for="(v, i) in values" :key="i" class="flex min-w-0 flex-col gap-1">
      <div class="flex justify-between font-mono text-[11px] tabular-nums text-gray-400">
        <span>{{ labels?.[i] ?? `C${i}` }}</span>
        <span class="text-gray-200">{{ v.toFixed(decimals) }}{{ unit }}</span>
      </div>
      <div class="h-1 overflow-hidden rounded bg-white/10">
        <div
          class="h-full rounded transition-all duration-500"
          :class="barClass"
          :style="{ width: Math.min(100, (v / max) * 100) + '%' }"
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

const props = withDefaults(defineProps<{
  values: number[]
  max: number
  unit: string
  barClass: string
  decimals?: number
  maxCols?: number
  labels?: string[]
}>(), { decimals: 0, maxCols: 4 })

// Pick the fewest rows that fit within maxCols, then spread items evenly across them.
// 8 cores, maxCols 4 -> 2 rows x 4 cols
// 12 cores, maxCols 4 -> 3 rows x 4 cols
// 6 cores, maxCols 4 -> 2 rows x 3 cols
const cols = computed(() => {
  const n = props.values.length
  if (n === 0) return 1
  const rows = Math.ceil(n / props.maxCols)
  return Math.ceil(n / rows)
})
</script>
