<template>
  <!-- Single wrapper element. Width transitions dynamically based on state -->
  <div
    class="sticky top-0 h-screen bg-mist-800/30 flex flex-col transition-all duration-200 ease-in-out overflow-hidden"
    :class="showExtendedLeftPanel ? 'w-40 p-3' : 'w-10 pt-3 items-center'"
  >
    <div v-if="showExtendedLeftPanel" class="flex flex-col w-full h-full">
      <div class="flex flex-row justify-end items-center">
        <button @click="panelControl" class="hover:opacity-80 cursor-pointer">
          <PanelLeftClose :size="20"/>
        </button>
      </div>
      <ul class="list-none flex flex-col gap-3 pt-20" key="menu">
        <li
          v-for="item in menuItems"
          :key="item.id"
          class="flex flex-row gap-5 items-center p-2 cursor-pointer"
          :class="item.id == props.selected ? 'bg-mist-300/30 w-full rounded-xl' : ''"
          @click="emit('selected', item.id)"
        >
          <Component :is="item.icon" :size="18"/>
          <span>
            {{ item.label }}
          </span>
        </li>
      </ul>
    </div>

    <div v-else class="flex flex-col items-center w-full h-full">
      <button @click="panelControl" class="hover:opacity-80 cursor-pointer">
        <PanelLeftOpen :size="20"/>
      </button>
      <ul class="list-none flex flex-col gap-3 pt-20" key="menu">
        <li
          v-for="item in menuItems"
          :key="item.id"
          class="flex flex-row gap-5 items-center p-2"
          @click="emit('selected', item.id)"
          :class="item.id == props.selected ? 'bg-mist-300/30 w-full rounded-xl' : ''"
        >
          <Component :is="item.icon" :size="18"/>
        </li>
      </ul>
    </div>
  </div>
</template>
<script setup lang="ts">
import { PanelLeftClose, PanelLeftOpen, Cpu, MemoryStick, Gpu, HardDrive, DiagramFlowHorizontal , Network, Settings } from '@lucide/vue';
import { ref } from 'vue';

const emit = defineEmits < {
  selected: [id: string]
}>()

const props = defineProps<{
  selected: string
}>()


const showExtendedLeftPanel = ref(localStorage.getItem("panel_extended") ?? false);
const menuItems = [
  { id: 'cpu', label: 'CPU', icon: Cpu },
  { id: 'ram', label: 'RAM', icon: MemoryStick },
  { id: 'gpu', label: 'GPU', icon: Gpu },
  { id: 'drives', label: 'Drives', icon: HardDrive },
  { id: 'network', label: 'Networks', icon: Network },
  { id: 'processes', label: 'Processes', icon: DiagramFlowHorizontal },
  { id: 'settings', label: 'Settings', icon: Settings},
];

function panelControl() : void {
  showExtendedLeftPanel.value = !showExtendedLeftPanel.value
}

</script>
