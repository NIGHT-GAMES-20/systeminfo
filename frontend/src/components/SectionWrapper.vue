<script lang="ts">
// Module-level: shared by every Section instance
let lockUntilScrollEnd = false
let lastEmittedId = ''
</script>

<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted } from 'vue'

interface Props {
  id: string
  activeSection?: string
}

const props = withDefaults(defineProps<Props>(), {
  activeSection: ''
})

const emit = defineEmits<{
  (e: 'section-enter', id: string): void
}>()

const sectionRef = ref<HTMLElement | null>(null)
let observer: IntersectionObserver | null = null

function unlockAfterScroll() {
  let done = false
  const finish = () => {
    if (done) return
    done = true
    clearTimeout(timer)
    window.removeEventListener('scrollend', finish, true)
    lockUntilScrollEnd = false
  }
  const timer = setTimeout(finish, 1200) // fallback if scrollend isn't supported
  window.addEventListener('scrollend', finish, true)
}

// Scroll only when the change came from outside (e.g. a nav click),
// never when it came from the user scrolling.
watch(
  () => props.activeSection,
  (newVal) => {
    if (newVal !== props.id || !sectionRef.value) return
    if (newVal === lastEmittedId) return // originated from scrolling, ignore

    const rect = sectionRef.value.getBoundingClientRect()
    if (Math.abs(rect.top) <= 100) return // already there

    lastEmittedId = props.id
    lockUntilScrollEnd = true
    sectionRef.value.scrollIntoView({ behavior: 'smooth', block: 'start' })
    unlockAfterScroll()
  }
)

onMounted(() => {
  observer = new IntersectionObserver(
    ([entry]) => {
      if (!entry?.isIntersecting) return
      if (lockUntilScrollEnd) return
      if (lastEmittedId === props.id) return
      lastEmittedId = props.id
      emit('section-enter', props.id)
    },
    {
      // Active = the section crossing the middle of the viewport.
      // Works for any section height, and a section that is only
      // just peeking in at the edge doesn't count.
      rootMargin: '-45% 0px -45% 0px',
      threshold: 0
    }
  )

  if (sectionRef.value) observer.observe(sectionRef.value)
})

onUnmounted(() => {
  observer?.disconnect()
  observer = null
})
</script>

<template>
  <section ref="sectionRef" :id="id" class="w-full h-full">
    <slot />
  </section>
</template>
