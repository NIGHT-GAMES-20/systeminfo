<template>
  <Teleport to="body">
    <Transition
      enter-active-class="transition duration-200 ease-out"
      enter-from-class="opacity-0"
      enter-to-class="opacity-100"
      leave-active-class="transition duration-150 ease-in"
      leave-from-class="opacity-100"
      leave-to-class="opacity-0"
    >
      <div
        v-if="props.isOpen"
        @click.self="emit('exit')"
        class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-md"
      >
        <div
          class="relative flex w-full max-w-md flex-col overflow-hidden rounded-2xl border border-white/10 bg-[#1a2224]/95 text-white shadow-2xl ring-1 ring-black/50"
        >
          <!-- Header -->
          <div class="flex items-center justify-between border-b border-white/10 px-6 py-4">
            <div class="flex items-center gap-2.5">
              <Server :size="18" class="text-teal-400" />
              <h2 class="text-base font-semibold tracking-wide text-gray-100">
                Select Node
              </h2>
            </div>

            <div class="flex items-center gap-1.5">
              <button
                type="button"
                title="Add new node"
                @click="() => { replace = false; modal_open = true }"
                class="flex items-center gap-1.5 rounded-lg bg-teal-500/10 px-2.5 py-1.5 text-xs font-medium text-teal-300 transition hover:bg-teal-500/20 hover:text-teal-200 active:scale-95"
              >
                <Plus :size="15" />
                <span>Add</span>
              </button>
              <button
                type="button"
                @click="emit('exit')"
                title="Close"
                class="rounded-lg p-1.5 text-gray-400 transition hover:bg-white/10 hover:text-white active:scale-95"
              >
                <X :size="18" />
              </button>
            </div>
          </div>

          <!-- Node List -->
          <div class="flex max-h-80 flex-col gap-2.5 overflow-y-auto p-4 custom-scrollbar">
            <div
              v-for="(endpoint, name) in available_nodes"
              :key="name"
              @click="selectNode(name)"
              class="group relative flex cursor-pointer items-center justify-between rounded-xl border p-3.5 transition-all duration-150"
              :class="
                selectedNode === name
                  ? 'border-teal-500/50 bg-teal-500/10 shadow-[inset_0_1px_0_0_rgba(255,255,255,0.08)]'
                  : 'border-white/5 bg-white/3 hover:border-white/15 hover:bg-white/[0.07]'
              "
            >
              <div class="flex items-center gap-3.5">
                <!-- Status / Active Indicator -->
                <div class="relative flex h-2.5 w-2.5 shrink-0 items-center justify-center">
                  <span
                    v-if="selectedNode === name"
                    class="absolute inline-flex h-full w-full animate-[ping_3s_cubic-bezier(0,0,0.2,1)_infinite] rounded-full bg-teal-400 opacity-75"
                  />
                  <span
                    class="relative inline-flex h-2 w-2 rounded-full transition-colors"
                    :class="selectedNode === name ? 'bg-teal-400' : 'bg-gray-600 group-hover:bg-gray-400'"
                  />
                </div>

                <!-- Node Info -->
                <div class="flex flex-col truncate">
                  <span
                    class="truncate text-sm font-semibold tracking-tight transition-colors"
                    :class="selectedNode === name ? 'text-teal-300' : 'text-gray-200 group-hover:text-white'"
                  >
                    {{ name }}
                  </span>
                  <span class="truncate font-mono text-xs text-gray-400">
                    {{ endpoint }}
                  </span>
                </div>
              </div>

              <!-- Actions -->
              <div class="flex items-center gap-2 pl-3">
                <span
                  v-if="selectedNode === name"
                  class="rounded-md bg-teal-500/20 px-2 py-0.5 text-[10px] font-medium uppercase tracking-wider text-teal-300"
                >
                  Active
                </span>
                <button
                  type="button"
                  @click.stop="() => {
                    addition.name = name;
                    addition.endpoint = endpoint;
                    replace = true;
                    modal_open = true;
                  }"
                  title="Edit node"
                  class="rounded-lg p-1.5 text-gray-400 opacity-70 transition hover:bg-white/10 hover:text-white group-hover:opacity-100"
                >
                  <Pencil :size="15" />
                </button>
                <button
                  type="button"
                  @click.stop="deleteNode(name)"
                  v-if="name != 'default'"
                  title="Edit node"
                  class="rounded-lg p-1.5 text-gray-400 opacity-70 transition hover:bg-white/10 hover:text-white group-hover:opacity-100"
                >
                  <Trash :size="15" />
                </button>
              </div>
            </div>
            <button
              type="button"
              title="Reload page"
              @click="reload"
              class="flex items-center gap-1.5 self-end w-fit rounded-lg bg-teal-500/10 px-2.5 py-1.5 text-xs font-medium text-teal-300 transition hover:bg-teal-500/20 hover:text-teal-200 active:scale-95"
            >
              <RefreshCcw :size="15" />
              Reload
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
  <Teleport to="body" v-if="modal_open">
    <Transition
      enter-active-class="transition duration-200 ease-out"
      enter-from-class="opacity-0"
      enter-to-class="opacity-100"
      leave-active-class="transition duration-150 ease-in"
      leave-from-class="opacity-100"
      leave-to-class="opacity-0"
    >
      <div @click.self="() => modal_open = false" class="fixed inset-0 z-50 flex items-center justify-center p-4 backdrop-blur-sm">
        <div class="relative flex w-full max-w-sm flex-col overflow-hidden rounded-2xl border border-white/10 bg-[#1a2224]/95 text-white shadow-2xl ring-1 ring-black/50">
          <div class="flex items-center justify-between border-b border-white/10 px-6 py-4">
            <div class="flex items-center gap-2.5">
              <Pencil :size="18" class="text-teal-400" />
              <h2 class="text-base font-semibold tracking-wide text-gray-100">
                Add/Edit Node
              </h2>
            </div>

            <div class="flex items-center gap-1.5">
              <button
                type="button"
                @click="() => modal_open = false"
                title="Close"
                class="rounded-lg p-1.5 text-gray-400 transition hover:bg-white/10 hover:text-white active:scale-95"
              >
                <X :size="18" />
              </button>
            </div>
          </div>
          <form @submit.prevent="handleNodeSubmit()" class="flex flex-col p-4 gap-4">
            <div class="flex flex-col gap-4">
              <div class="flex flex-col gap-1">
                <label for="name" class="text-teal-200 font-bold text-xs">Name: </label>
                <input id="name" type="text" v-model.trim="addition.name" placeholder="xyz" required class="rounded-lg bg-teal-500/10 text-teal-200 p-1 text-xs focus:outline-none "/>
              </div>
              <div class="flex flex-col gap-1">
                <label for="name" class="text-teal-200 font-bold text-xs">Endpoint: </label>
                <input id="name" type="text" v-model.trim="addition.endpoint" placeholder="http://xyz, https://xyz, 169.29.2.14" required class="rounded-lg bg-teal-500/10 text-teal-200 p-1 text-xs focus:outline-none" />
              </div>
            </div>
            <button type="submit" class="w-fit rounded-lg bg-teal-500/10 px-2.5 py-1.5 text-xs font-medium text-teal-300 self-center">Add Node</button>
          </form>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { reactive, ref } from 'vue';
import { X, Pencil, Plus, Server, RefreshCcw, Trash } from '@lucide/vue';

const available_nodes = ref<Record<string, string>>(
  JSON.parse(localStorage.getItem('available_nodes') ?? '{"default": "/systeminfo/api"}')
);

const selectedNode = ref<string>(localStorage.getItem('selected_node') ?? 'default');

const props = defineProps({
  isOpen: {
    type: Boolean,
    required: true
  }
});

const modal_open = ref(false)
const replace = ref(false)

const addition = reactive({
  name: "",
  endpoint: "",
})

const getUniqueNodeName = (rawName: string): string => {
  let candidate = rawName

  // Keep incrementing until we find a name that doesn't exist
  while (Object.hasOwn(available_nodes.value, candidate)) {
    const parts = candidate.split('#')
    const lastPart = parts[parts.length - 1]
    const parsedNumber = Number(lastPart)

    // Check if there is a '#' AND the suffix is a valid number
    if (parts.length > 1 && lastPart.trim() !== '' && !Number.isNaN(parsedNumber)) {
      const basePart = parts.slice(0, -1).join('#')
      candidate = `${basePart} #${parsedNumber + 1}`
    } else {
      candidate = `${candidate} #1`
    }
  }

  return candidate
}


const handleNodeSubmit = () => {
  const finalName = replace.value ? addition.name : getUniqueNodeName(addition.name)

  available_nodes.value[finalName] = addition.endpoint
  localStorage.setItem('available_nodes', JSON.stringify(available_nodes.value))

  // Optional: reset form inputs after saving
  addition.name = ''
  addition.endpoint = ''
  modal_open.value = false
}

const deleteNode = (name: string) => {

  if (name == selectedNode.value) {
    selectedNode.value = "default"
  }

  if (name == "default") {
    return
  }

  delete available_nodes.value[name];
  localStorage.setItem('available_nodes', JSON.stringify(available_nodes.value))
}

const reload = () => {
  window.location.reload()
}


const emit = defineEmits(['exit', 'select']);

const selectNode = (name: string) => {
  selectedNode.value = name;
  localStorage.setItem('selected_node', name);
  emit('select', { name, endpoint: available_nodes.value[name] });
};
</script>

<style scoped>
/* Firefox */
.custom-scrollbar {
  scrollbar-width: thin;
  scrollbar-color: rgba(45, 212, 191, 0.3) transparent; /* teal-400/30 thumb, transparent track */
}

/* Chrome, Edge, and Safari */
.custom-scrollbar::-webkit-scrollbar {
  width: 6px;
}

.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}

.custom-scrollbar::-webkit-scrollbar-thumb {
  background-color: rgba(45, 212, 191, 0.25);
  border-radius: 9999px;
}

.custom-scrollbar::-webkit-scrollbar-thumb:hover {
  background-color: rgba(45, 212, 191, 0.5);
}
</style>
