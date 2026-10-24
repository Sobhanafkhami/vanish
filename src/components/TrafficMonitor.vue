<script setup lang="ts">
import { t } from "../lib/i18n";
import { computed } from "vue";
import { ArrowDown, ArrowUp } from "lucide-vue-next";
import { store } from "../stores/app";

const W = 300;
const H = 64;

function fmt(bytes: number, suffix = "/s") {
  const u = ["B", "KB", "MB", "GB", "TB"];
  let i = 0;
  while (bytes >= 1024 && i < u.length - 1) {
    bytes /= 1024;
    i++;
  }
  return `${bytes.toFixed(i ? 1 : 0)} ${u[i]}${suffix}`;
}

const scale = computed(() => Math.max(1024, ...store.upHistory, ...store.downHistory));
function path(data: number[]) {
  const step = W / (data.length - 1);
  return data.map((v, i) => `${i ? "L" : "M"}${(i * step).toFixed(1)},${(H - (v / scale.value) * (H - 4) - 2).toFixed(1)}`).join(" ");
}
const downPath = computed(() => path(store.downHistory));
const upPath = computed(() => path(store.upHistory));
</script>

<template>
  <div class="panel shrink-0 p-4">
    <div class="mb-3 flex items-center justify-between">
      <span class="hud-label">{{ t("Live traffic") }}</span>
      <span class="hud-label">{{ t("peak") }} {{ fmt(scale) }}</span>
    </div>
    <svg :viewBox="`0 0 ${W} ${H}`" class="h-16 w-full" preserveAspectRatio="none">
      <path :d="`${downPath} L${W},${H} L0,${H} Z`" fill="rgba(34,197,94,.12)" />
      <path :d="downPath" fill="none" stroke="#4ade80" stroke-width="1.5" vector-effect="non-scaling-stroke" />
      <path :d="upPath" fill="none" stroke="#a1a8a3" stroke-width="1.5" vector-effect="non-scaling-stroke" />
    </svg>
    <div class="mt-3 grid grid-cols-2 gap-3 font-mono">
      <div>
        <div class="flex items-center gap-1 text-[10px] text-accent-soft"><ArrowDown :size="12" /> {{ t("DOWN") }}</div>
        <div class="text-lg text-white">{{ fmt(store.down) }}</div>
        <div class="text-[10px] text-white/50">Σ {{ fmt(store.downTotal, "") }}</div>
      </div>
      <div>
        <div class="flex items-center gap-1 text-[10px] text-white/60"><ArrowUp :size="12" /> {{ t("UP") }}</div>
        <div class="text-lg text-white">{{ fmt(store.up) }}</div>
        <div class="text-[10px] text-white/50">Σ {{ fmt(store.upTotal, "") }}</div>
      </div>
    </div>
  </div>
</template>
