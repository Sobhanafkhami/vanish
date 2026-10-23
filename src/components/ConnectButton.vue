<script setup lang="ts">
import { computed } from "vue";
import { Power } from "lucide-vue-next";
import { store, toggleConnection, selectedNode } from "../stores/app";

const on = computed(() => store.status.running);
const label = computed(() => (store.busy ? "WORKING" : on.value ? "CONNECTED" : "OFFLINE"));
</script>

<template>
  <div class="flex flex-col items-center gap-4">
    <div class="relative grid h-44 w-44 place-items-center">
      <!-- ping pulse rings while connected -->
      <template v-if="on">
        <span class="absolute inset-4 animate-ping-pulse rounded-full border border-cyan-neon/60"></span>
        <span class="absolute inset-4 animate-ping-pulse rounded-full border border-violet-neon/60" style="animation-delay: 0.9s"></span>
      </template>
      <!-- rotating HUD ring -->
      <svg class="absolute inset-0 h-full w-full" :class="{ 'animate-[spin_14s_linear_infinite]': on }" viewBox="0 0 100 100">
        <circle cx="50" cy="50" r="47" fill="none" stroke="url(#g)" stroke-width="1" stroke-dasharray="2 4" opacity=".7" />
        <circle cx="50" cy="50" r="42" fill="none" stroke="url(#g)" stroke-width="1.5" stroke-dasharray="60 204" stroke-linecap="round" />
        <defs>
          <linearGradient id="g" x1="0" y1="0" x2="1" y2="1">
            <stop offset="0" stop-color="#00F2FE" />
            <stop offset="1" stop-color="#7000FF" />
          </linearGradient>
        </defs>
      </svg>
      <button
        :disabled="store.busy || !selectedNode"
        class="relative grid h-28 w-28 place-items-center rounded-full border backdrop-blur-xl transition-all duration-500 disabled:cursor-not-allowed disabled:opacity-50"
        :class="
          on
            ? 'border-cyan-neon bg-cyan-neon/10 text-cyan-neon shadow-[0_0_40px_rgba(0,242,254,.45)]'
            : 'border-white/20 bg-white/5 text-white/60 hover:border-violet-neon hover:text-white hover:shadow-glow-violet'
        "
        @click="toggleConnection"
      >
        <Power :size="40" :stroke-width="1.6" />
      </button>
    </div>
    <div class="text-center">
      <div class="font-mono text-sm font-semibold tracking-[0.35em]" :class="on ? 'text-cyan-neon' : 'text-white/40'">{{ label }}</div>
      <div class="mt-1 max-w-[16rem] truncate text-xs text-white/50">
        {{ selectedNode ? selectedNode.name : "Select or import a node" }}
      </div>
    </div>
  </div>
</template>
