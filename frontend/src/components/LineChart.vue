<template>
  <div class="flex flex-col">
    <!-- Hidden probe elements to convert Tailwind classes into RGB strings -->
    <span ref="strokeProbe" class="hidden" :class="strokeClass"></span>
    <span ref="gridProbe" class="hidden" :class="gridClass"></span>
    <span ref="tickProbe" class="hidden" :class="tickClass"></span>

    <!-- Optional Header -->
    <slot name="header" :latest="latestValue" :count="recordedCount">
      <div v-if="title || showValue" :class="headerClass">
        <div>
          <h3 v-if="title" :class="titleClass">{{ title }}</h3>
          <p v-if="showSubtext" :class="subtextClass">
            {{ recordedCount }} / {{ maxItems }} samples
          </p>
        </div>
        <span v-if="showValue" :class="valueClass">
          {{ latestValue }}{{ unit }}
        </span>
      </div>
    </slot>

    <!-- Chart Canvas Wrapper -->
    <div class="relative w-full" :class="chartWrapperClass">
      <Line :data="chartData" :options="chartOptions" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onBeforeUnmount } from 'vue'
import { Line } from 'vue-chartjs'
import {
  Chart as ChartJS,
  Title,
  Tooltip,
  Legend,
  LineElement,
  PointElement,
  CategoryScale,
  LinearScale,
  Filler,
  type ChartData,
  type ChartOptions
} from 'chart.js'

ChartJS.register(
  Title,
  Tooltip,
  Legend,
  LineElement,
  PointElement,
  CategoryScale,
  LinearScale,
  Filler
)

interface Props {
  latestValue: number
  pollInterval?: number // ms between samples. 0 = sample only when latestValue changes
  title?: string
  unit?: string
  maxItems?: number
  minScale?: number
  maxScale?: number
  yAxisPosition?: 'left' | 'right' // Put Y-axis on right or left
  tension?: number
  fillArea?: boolean
  showPoints?: boolean
  showValue?: boolean
  showSubtext?: boolean

  // Tailwind Class Props
  headerClass?: string
  titleClass?: string
  subtextClass?: string
  valueClass?: string
  chartWrapperClass?: string
  strokeClass?: string
  gridClass?: string
  tickClass?: string
}

const props = withDefaults(defineProps<Props>(), {
  pollInterval: 1000,
  title: '',
  unit: '%',
  maxItems: 40,
  minScale: 0,
  yAxisPosition: 'left',
  tension: 0.35,
  fillArea: true,
  showPoints: false,
  showValue: true,
  showSubtext: true,

  headerClass: 'flex items-center justify-between mb-3',
  titleClass: 'text-sm font-semibold text-slate-200',
  subtextClass: 'text-xs text-slate-500',
  valueClass: 'text-xl font-bold font-mono text-sky-400',
  chartWrapperClass: 'h-48',
  strokeClass: 'text-sky-400',
  gridClass: 'text-slate-800',
  tickClass: 'text-slate-500'
})

// Raw internal history (only holds actual received readings, up to maxItems)
const rawValues = ref<number[]>([])
const rawLabels = ref<string[]>([])

const recordedCount = computed(() => rawValues.value.length)

// Fixed-length padded arrays: nulls on the left, live data pinned to the right (end)
const paddedData = computed(() => {
  const emptyCount = Math.max(0, props.maxItems - rawValues.value.length)
  const emptyValues = Array<number | null>(emptyCount).fill(null)
  const emptyLabels = Array<string>(emptyCount).fill('')

  return {
    values: [...emptyValues, ...rawValues.value],
    labels: [...emptyLabels, ...rawLabels.value]
  }
})

// Hidden probes for Tailwind color extraction
const strokeProbe = ref<HTMLElement | null>(null)
const gridProbe = ref<HTMLElement | null>(null)
const tickProbe = ref<HTMLElement | null>(null)

const resolvedStroke = ref('rgb(56, 189, 248)')
const resolvedGrid = ref('rgb(30, 41, 59)')
const resolvedTick = ref('rgb(100, 116, 139)')

const toAlphaColor = (cssColor: string, alpha: number) => {
  if (cssColor.startsWith('rgb(')) {
    return cssColor.replace('rgb(', 'rgba(').replace(')', `, ${alpha})`)
  }
  return `color-mix(in srgb, ${cssColor} ${alpha * 100}%, transparent)`
}

const syncTailwindColors = () => {
  if (strokeProbe.value) resolvedStroke.value = getComputedStyle(strokeProbe.value).color
  if (gridProbe.value) resolvedGrid.value = getComputedStyle(gridProbe.value).color
  if (tickProbe.value) resolvedTick.value = getComputedStyle(tickProbe.value).color
}

watch(() => [props.strokeClass, props.gridClass, props.tickClass], syncTailwindColors, {
  flush: 'post'
})

const pushReading = (val: number) => {
  const timeLabel = new Date().toLocaleTimeString([], {
    minute: '2-digit',
    second: '2-digit'
  })

  rawValues.value.push(val)
  rawLabels.value.push(timeLabel)

  if (rawValues.value.length > props.maxItems) {
    rawValues.value.shift()
    rawLabels.value.shift()
  }
}

const clearHistory = () => {
  rawValues.value = []
  rawLabels.value = []
}

// ---- Own polling loop -------------------------------------------------------
// Samples the current value on a fixed interval, whether or not it changed,
// so flat metrics (e.g. unused swap at 0%) keep scrolling.
let timer: ReturnType<typeof setInterval> | null = null

const sample = () => {
  const v = props.latestValue
  if (typeof v === 'number' && !Number.isNaN(v)) pushReading(v)
}

const stopPolling = () => {
  if (timer !== null) {
    clearInterval(timer)
    timer = null
  }
}

const startPolling = () => {
  stopPolling()
  if (props.pollInterval > 0) {
    timer = setInterval(sample, props.pollInterval)
  }
}

onMounted(() => {
  syncTailwindColors()
  sample() // first point immediately so the chart isn't empty
  startPolling()
})

onBeforeUnmount(stopPolling)

// Restart the timer if the interval prop changes
watch(() => props.pollInterval, startPolling)

// Legacy mode: pollInterval = 0 -> sample only when the value changes
watch(
  () => props.latestValue,
  () => {
    if (props.pollInterval <= 0) sample()
  }
)

defineExpose({ pushReading, clearHistory })

const chartData = computed<ChartData<'line'>>(() => ({
  labels: paddedData.value.labels,
  datasets: [
    {
      label: props.title || 'Metric',
      data: paddedData.value.values,
      borderColor: resolvedStroke.value,
      backgroundColor: props.fillArea
        ? toAlphaColor(resolvedStroke.value, 0.18)
        : 'transparent',
      borderWidth: 2,
      tension: props.tension,
      fill: props.fillArea,
      spanGaps: false,
      pointRadius: (ctx) => {
        // Show a visible dot when only 1 reading exists on the far right edge
        if (recordedCount.value === 1 && ctx.raw !== null) return 3
        return props.showPoints ? 3 : 0
      },
      pointHoverRadius: 5,
      pointBackgroundColor: resolvedStroke.value
    }
  ]
}))

const chartOptions = computed<ChartOptions<'line'>>(() => ({
  responsive: true,
  maintainAspectRatio: false,
  animation: { duration: 150 },
  interaction: {
    mode: 'index',
    intersect: false
  },
  scales: {
    x: {
      grid: { display: false },
      ticks: {
        color: resolvedTick.value,
        maxTicksLimit: 6,
        font: { size: 10 }
      }
    },
    y: {
      position: props.yAxisPosition,
      min: props.minScale,
      max: props.maxScale,
      grace: props.maxScale === undefined ? '10%' : 0,
      grid: { color: resolvedGrid.value },
      ticks: {
        color: resolvedTick.value,
        callback: (val) => `${val}${props.unit}`
      }
    }
  },
  plugins: {
    legend: { display: false },
    tooltip: {
      filter: (tooltipItem) => tooltipItem.raw !== null,
      callbacks: {
        label: (ctx) => ` ${ctx.parsed.y}${props.unit}`
      }
    }
  }
}))
</script>
