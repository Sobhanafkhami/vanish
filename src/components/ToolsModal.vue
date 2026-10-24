<script setup lang="ts">
import { computed, ref } from "vue";
import { CircleCheck, CircleX, Info, Play, TriangleAlert } from "lucide-vue-next";
import Modal from "./Modal.vue";
import { runDiagnostics, runSpeedTest, store } from "../stores/app";

const tab = ref<"speed" | "leak">("speed");
const tabs = [
  { id: "speed", label: "Speed test" },
  { id: "leak", label: "Leak test" },
] as const;

const steps = ["ping", "download", "upload"] as const;
const stepIndex = computed(() => steps.indexOf(store.speed.phase as (typeof steps)[number]));
const phaseLabel = computed(() => {
  const p = store.speed.phase;
  return p === "ping" ? "Measuring latency" : p === "download" ? "Download" : p === "upload" ? "Upload" : "";
});

const fmt = (n: number, d = 1) => n.toFixed(d);
const r = computed(() => store.speed.result);
const big = computed(() => {
  if (store.speed.running) return store.speed.phase === "ping" ? "0.0" : fmt(store.speed.live);
  return r.value ? fmt(r.value.download_mbps) : "0.0";
});
const metrics = computed(() => [
  { k: "Ping", v: r.value ? fmt(r.value.ping_ms, 0) : "—", u: "ms" },
  { k: "Jitter", v: r.value ? fmt(r.value.jitter_ms, 0) : "—", u: "ms" },
  { k: "Download", v: r.value ? fmt(r.value.download_mbps) : "—", u: "Mbit/s" },
  { k: "Upload", v: r.value ? fmt(r.value.upload_mbps) : "—", u: "Mbit/s" },
]);
const hint = computed(() => {
  if (r.value) return r.value.via_tunnel ? "Measured through the active tunnel." : "Measured on your direct connection (not connected).";
  return store.status.running ? "Runs through the active tunnel. Takes about 20 seconds." : "Not connected: this tests your direct connection. Takes about 20 seconds.";
});

const icons = { ok: CircleCheck, warn: TriangleAlert, fail: CircleX, info: Info } as const;
const tone = { ok: "text-accent-soft", warn: "text-yellow-300", fail: "text-red-400", info: "text-white/55" } as const;
const word = { ok: "OK", warn: "Warning", fail: "Problem", info: "Info" } as const;
</script>

<template>
  <Modal title="Network tools" wide @close="store.ui.tools = false">
    <div class="mb-4 flex gap-1 rounded-lg border border-ink-line bg-black/40 p-1" role="tablist">
      <button
        v-for="t in tabs"
        :key="t.id"
        role="tab"
        :aria-selected="tab === t.id"
        class="flex-1 rounded-md px-3 py-1.5 text-xs transition"
        :class="tab === t.id ? 'bg-accent/15 text-accent-soft' : 'text-white/50 hover:text-white/80'"
        @click="tab = t.id"
      >
        {{ t.label }}
      </button>
    </div>

    <!-- speed test -->
    <div v-if="tab === 'speed'">
      <div class="flex flex-col items-center gap-1 py-4">
        <div class="font-mono text-5xl font-semibold text-white" aria-live="polite">{{ big }}</div>
        <div class="text-xs text-white/50">Mbit/s {{ phaseLabel ? "· " + phaseLabel : "" }}</div>
        <div class="mt-3 flex gap-2" aria-hidden="true">
          <span
            v-for="(s, i) in steps"
            :key="s"
            class="h-1 w-12 rounded-full transition-colors"
            :class="store.speed.running && i <= stepIndex ? 'bg-accent' : r && !store.speed.running ? 'bg-accent/60' : 'bg-white/10'"
          ></span>
        </div>
      </div>

      <div class="grid grid-cols-4 gap-3 text-center">
        <div v-for="m in metrics" :key="m.k" class="rounded-xl border border-ink-line bg-black/30 px-2 py-3">
          <div class="hud-label">{{ m.k }}</div>
          <div class="mt-1 font-mono text-lg text-white">{{ m.v }}</div>
          <div class="text-[10px] text-white/45">{{ m.u }}</div>
        </div>
      </div>

      <p v-if="store.speed.error" class="mt-3 rounded-lg border border-red-500/40 p-2 font-mono text-xs text-red-300" role="alert">{{ store.speed.error }}</p>
      <p class="mt-3 text-xs text-white/50">{{ hint }}</p>
      <button class="btn mt-3" :disabled="store.speed.running" @click="runSpeedTest">
        <Play :size="13" /> {{ store.speed.running ? "Testing…" : r ? "Run again" : "Start" }}
      </button>
    </div>

    <!-- leak test -->
    <div v-else>
      <p class="mb-3 text-xs text-white/50">Compares your direct connection with the tunnel: exit IP, DNS resolver and IPv6.</p>
      <ul v-if="store.diag.result" class="space-y-2">
        <li v-for="c in store.diag.result.checks" :key="c.label" class="flex items-start gap-3 rounded-xl border border-ink-line bg-black/30 px-3 py-2.5">
          <component :is="icons[c.status]" :size="16" class="mt-0.5 shrink-0" :class="tone[c.status]" />
          <div class="min-w-0">
            <div class="text-sm text-white/90">
              {{ c.label }} <span class="ml-1 text-[10px] uppercase tracking-wider" :class="tone[c.status]">{{ word[c.status] }}</span>
            </div>
            <div class="break-words text-xs text-white/55" style="user-select: text">{{ c.detail }}</div>
          </div>
        </li>
      </ul>
      <p v-if="store.diag.error" class="rounded-lg border border-red-500/40 p-2 font-mono text-xs text-red-300" role="alert">{{ store.diag.error }}</p>
      <button class="btn mt-3" :disabled="store.diag.running" @click="runDiagnostics">
        <Play :size="13" /> {{ store.diag.running ? "Checking…" : store.diag.result ? "Run again" : "Run test" }}
      </button>
    </div>
  </Modal>
</template>
