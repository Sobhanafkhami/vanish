<script setup lang="ts">
import { computed } from "vue";
import { Power } from "lucide-vue-next";
import { store, toggleConnection, selectedNode } from "../stores/app";

const on = computed(() => store.status.running);
const label = computed(() => (store.busy ? "Working" : on.value ? "Connected" : "Offline"));
</script>

<template>
  <div class="flex flex-col items-center gap-4">
    <div class="relative grid h-36 w-36 place-items-center">
      <!-- green backdrop, only while connected -->
      <span v-if="on" class="absolute inset-3 animate-backdrop-pulse rounded-full bg-accent/40 blur-2xl"></span>
      <button
        :disabled="store.busy || !selectedNode"
        :aria-pressed="on"
        :aria-label="on ? 'Disconnect' : 'Connect'"
        class="relative grid h-24 w-24 place-items-center rounded-full border transition-colors duration-300 disabled:opacity-50"
        :class="on ? 'border-accent/60 bg-ink text-accent-soft' : 'border-ink-line bg-ink-raised text-white/60 hover:text-white'"
        @click="toggleConnection"
      >
        <Power :size="34" :stroke-width="1.8" />
      </button>
    </div>
    <div class="text-center">
      <div class="text-sm font-semibold tracking-wide" :class="on ? 'text-accent-soft' : 'text-white/50'">{{ label }}</div>
      <div class="mt-1 max-w-[16rem] truncate text-xs text-white/50">
        {{ selectedNode ? selectedNode.name : "Select or import a node" }}
      </div>
    </div>
  </div>
</template>
