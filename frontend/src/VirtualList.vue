<script setup lang="ts" generic="T">
import { computed, onMounted, onUnmounted, ref, watch, nextTick } from 'vue'

const props = withDefaults(defineProps<{
  items: T[]
  itemHeight: number
  buffer?: number
  threshold?: number
}>(), {
  buffer: 5,
  threshold: 10,
})

const emit = defineEmits<{
  scrollEnd: []
}>()

const containerRef = ref<HTMLDivElement>()
const scrollTop = ref(0)
const containerHeight = ref(600)

let scrollTimer: ReturnType<typeof setTimeout> | null = null

const visibleRange = computed(() => {
  const start = Math.max(0, Math.floor(scrollTop.value / props.itemHeight) - props.buffer)
  const visibleCount = Math.ceil(containerHeight.value / props.itemHeight) + props.threshold
  const end = Math.min(props.items.length, start + visibleCount + props.buffer * 2)
  return { start, end }
})

const visibleItems = computed(() => {
  const { start, end } = visibleRange.value
  const result = []
  for (let i = start; i < end; i++) {
    result.push({
      item: props.items[i],
      index: i,
      style: {
        position: 'absolute' as const,
        top: `${i * props.itemHeight}px`,
        left: 0,
        right: 0,
        height: `${props.itemHeight}px`,
        overflow: 'hidden',
      },
    })
  }
  return result
})

const totalHeight = computed(() => props.items.length * props.itemHeight)

function onScroll(e: Event) {
  const el = e.target as HTMLDivElement
  scrollTop.value = el.scrollTop

  if (scrollTimer) clearTimeout(scrollTimer)
  scrollTimer = setTimeout(() => {
    if (el.scrollTop + el.clientHeight >= el.scrollHeight - 100) {
      emit('scrollEnd')
    }
  }, 150)
}

function updateHeight() {
  if (containerRef.value) {
    containerHeight.value = containerRef.value.clientHeight
  }
}

let resizeObserver: ResizeObserver | null = null

onMounted(async () => {
  await nextTick()
  updateHeight()
  resizeObserver = new ResizeObserver(() => {
    updateHeight()
  })
  if (containerRef.value) {
    resizeObserver.observe(containerRef.value)
  }
})

onUnmounted(() => {
  resizeObserver?.disconnect()
  if (scrollTimer) clearTimeout(scrollTimer)
})

watch(() => props.items.length, async () => {
  await nextTick()
  updateHeight()
})

function scrollToIndex(index: number) {
  if (containerRef.value) {
    containerRef.value.scrollTop = index * props.itemHeight
  }
}

defineExpose({ scrollToIndex })
</script>

<template>
  <div
    ref="containerRef"
    class="virtual-list"
    @scroll="onScroll"
  >
    <div
      class="virtual-list__spacer"
      :style="{ height: `${totalHeight}px` }"
    >
      <div
        v-for="entry in visibleItems"
        :key="entry.index"
        :style="entry.style"
        class="virtual-list__item"
      >
        <slot :item="entry.item" :index="entry.index" />
      </div>
    </div>
  </div>
</template>

<style scoped>
.virtual-list {
  position: relative;
  overflow-y: auto;
  overflow-x: hidden;
  height: 100%;
  width: 100%;
  box-sizing: border-box;
}

.virtual-list__spacer {
  position: relative;
}

.virtual-list__item {
  box-sizing: border-box;
  overflow: hidden;
}
</style>
