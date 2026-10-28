<script setup lang="ts">
import { t } from "../lib/i18n";
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
      <!-- sonar rings while connected, a spinning arc while connecting -->
      <template v-if="on && !store.busy">
        <span class="sonar absolute h-24 w-24 rounded-full border border-accent/40" aria-hidden="true"></span>
        <span class="sonar absolute h-24 w-24 rounded-full border border-accent/30" style="animation-delay: 1.2s" aria-hidden="true"></span>
      </template>
      <span v-if="store.busy" class="absolute h-[104px] w-[104px] animate-spin rounded-full border-2 border-transparent border-t-accent-soft" style="animation-duration: 0.9s" aria-hidden="true"></span>
      <button
        :disabled="store.busy || !selectedNode"
        :aria-pressed="on"
        :aria-label="t(on ? 'Disconnect' : 'Connect')"
        class="relative grid h-24 w-24 place-items-center rounded-full border transition-all duration-300 hover:scale-[1.04] active:scale-95 disabled:opacity-50 disabled:hover:scale-100"
        :class="on ? 'border-accent/60 bg-ink text-accent-soft' : 'border-ink-line bg-ink-raised text-white/60 hover:text-white'"
        @click="toggleConnection"
      >
        <Power :size="34" :stroke-width="1.8" />
      </button>
    </div>
    <div class="text-center">
      <div class="flex items-center justify-center gap-2 text-base font-semibold" :class="on ? 'text-accent-soft' : 'text-white/60'">
        <span class="h-2 w-2 rounded-full" :class="store.busy ? 'animate-pulse bg-yellow-300' : on ? 'bg-accent' : 'bg-white/25'" aria-hidden="true"></span>
        {{ t(label) }}
      </div>
      <div class="mt-1.5 max-w-[16rem] truncate text-sm" :class="selectedNode ? 'text-white/70' : 'text-white/40'">
        {{ selectedNode ? selectedNode.name : t("Select or import a node") }}
      </div>
    </div>
  </div>
</template>
