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
              <Settings :size="18" class="text-teal-400" />
              <h2 class="text-base font-semibold tracking-wide text-gray-100">
                Settings
              </h2>
            </div>
            <div class="flex items-center gap-1.5">
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
          <div class="flex max-h-80 flex-col gap-2.5 overflow-y-auto p-4 custom-scrollbar">
            <div class="group relative flex cursor-pointer items-center justify-between rounded-xl border p-3.5 border-white/5 bg-white/3">
              <form @submit.prevent="updateAddress()" class="flex flex-col gap-2 w-full">
                <div class="flex flex-row justify-between items-center">
                  <label class="text-teal-200 font-bold"> Address </label>
                  <button type="submit" class="w-fit rounded-lg bg-teal-500/10 px-2.5 py-1.5 text-xs font-medium text-teal-300 self-center">save</button>
                </div>
                <input id="address" type="text" v-model.trim="address.address" :placeholder="current_address" required class="rounded-lg bg-teal-500/10 text-teal-200 p-1 text-xs focus:outline-none "/>
              </form>
            </div>
            <div class="group relative flex cursor-pointer items-center justify-between rounded-xl border p-3.5 border-white/5 bg-white/3">
              <form @submit.prevent="updatePollingRate()" class="flex flex-col gap-2 w-full">
                <div class="flex flex-row justify-between items-center">
                  <label class="text-teal-200 font-bold"> Polling Rate </label>
                  <button type="submit" class="w-fit rounded-lg bg-teal-500/10 px-2.5 py-1.5 text-xs font-medium text-teal-300 self-center">save</button>
                </div>
                <input id="poll" type="number" v-model.trim="poll.poll" min="0" :placeholder="String(current_polling_rate)" required class="rounded-lg bg-teal-500/10 text-teal-200 p-1 text-xs focus:outline-none "/>
              </form>
            </div>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { reactive, ref } from 'vue';
import { X, Settings } from '@lucide/vue';

const current_address = ref(localStorage.getItem("target_address") ?? "London")
const current_polling_rate = ref(Number(localStorage.getItem("polling_interval") ?? 5))

const props = defineProps({
  isOpen: {
    type: Boolean,
    required: true
  }
});

const address = reactive({
  address: "",
})

const poll = reactive({
  poll: 0
})

const updateAddress = () => {
  localStorage.setItem('target_address', address.address)
  address.address = ""
  reload()
}

const updatePollingRate = () => {
  localStorage.setItem('polling_rate', String(poll.poll))
  poll.poll = 0
  reload()
}


const reload = () => {
  window.location.reload()
}

const emit = defineEmits(['exit']);

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
