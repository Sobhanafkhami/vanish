<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import Modal from "./Modal.vue";
import { store } from "../stores/app";

const level = ref<"all" | "warn" | "error">("all");
const box = ref<HTMLElement | null>(null);

const lines = computed(() =>
  store.logs.filter((l) => {
    if (level.value === "all") return true;
    const u = l.toUpperCase();
    return level.value === "error" ? /ERROR|FATAL|PANIC/.test(u) : /ERROR|FATAL|PANIC|WARN/.test(u);
  }),
);

function cls(l: string) {
  const u = l.toUpperCase();
  return /ERROR|FATAL|PANIC/.test(u) ? "text-red-400" : /WARN/.test(u) ? "text-yellow-300" : "text-white/70";
}

watch(
  () => lines.value.length,
  async () => {
    await nextTick();
    if (box.value) box.value.scrollTop = box.value.scrollHeight;
  },
  { immediate: true, flush: "post" },
);
</script>

<template>
  <Modal title="Core log" wide @close="store.ui.logs = false">
    <div class="mb-3 flex items-center justify-between">
      <div class="flex gap-1 rounded-lg border border-ink-line bg-black/40 p-1">
        <button
          v-for="l in ['all', 'warn', 'error'] as const"
          :key="l"
          class="rounded-md px-3 py-1 text-xs capitalize transition"
          :class="level === l ? 'bg-accent/15 text-accent-soft' : 'text-white/50 hover:text-white/80'"
          @click="level = l"
        >
          {{ l }}
        </button>
      </div>
      <button class="btn" @click="store.logs.length = 0">Clear</button>
    </div>
    <div ref="box" class="h-72 overflow-y-auto rounded-lg border border-ink-line bg-black/50 p-2 font-mono text-[11px] leading-5" style="user-select: text">
      <div v-if="!lines.length" class="text-white/40">No log lines yet.</div>
      <div v-for="(l, i) in lines" :key="i" class="whitespace-pre-wrap break-all" :class="cls(l)">{{ l }}</div>
    </div>
  </Modal>
</template>
