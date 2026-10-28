<script setup lang="ts">
import { t, tDetail } from "../lib/i18n";
import { computed, ref } from "vue";
import { CircleCheck, CircleX, Info, Play, TriangleAlert } from "lucide-vue-next";
import Modal from "./Modal.vue";
import SpeedGauge from "./SpeedGauge.vue";
import { runDiagnostics, runSpeedTest, store } from "../stores/app";

const tab = ref<"speed" | "leak">("speed");
const tabs = [
  { id: "speed", label: "Speed test" },
  { id: "leak", label: "Leak test" },
] as const;

const sp = computed(() => store.speed);
const r = computed(() => store.speed.result);
const phases = ["ping", "download", "upload"] as const;
const COLORS = { ping: "#4ade80", download: "#4ade80", upload: "#e4ebe6" } as const;

const idle = computed(() => !sp.value.running && !r.value && !sp.value.error);
const phaseKey = computed(() => (sp.value.running && sp.value.phase !== "done" && sp.value.phase !== "" ? sp.value.phase : "download"));
const gaugeColor = computed(() => COLORS[phaseKey.value]);
const gaugeValue = computed(() => {
  if (sp.value.running) return phaseKey.value === "ping" ? 0 : sp.value.live;
  return sp.value.partial.download ?? 0;
});
const caption = computed(() => (sp.value.running ? phaseKey.value.toUpperCase() : r.value ? "DOWNLOAD" : ""));

const fmt = (n: number | null, d = 1) => (n === null ? "—" : n.toFixed(d));
const cards = computed(() =>
  phases.map((p) => {
    const active = sp.value.running && sp.value.phase === p;
    const done = sp.value.partial[p];
    const v = p === "ping" ? done : active ? sp.value.live : done;
    return {
      p,
      active,
      label: p === "ping" ? "Ping" : p === "download" ? "Download" : "Upload",
      value: p === "ping" ? fmt(v, 0) : fmt(v === null || (!active && v === 0 && !r.value) ? null : v),
      unit: p === "ping" ? "ms" : "Mbit/s",
    };
  }),
);

// Live chart of the current phase.
const CHART_W = 300;
const CHART_H = 56;
const chart = computed(() => {
  const d = sp.value.samples;
  if (d.length < 2) return { line: "", area: "" };
  const max = Math.max(1, ...d);
  const step = CHART_W / (Math.max(d.length, 30) - 1);
  const pts = d.map((v, i) => `${(i * step).toFixed(1)},${(CHART_H - 3 - (v / max) * (CHART_H - 6)).toFixed(1)}`);
  return { line: "M" + pts.join(" L"), area: `M${pts.join(" L")} L${((d.length - 1) * step).toFixed(1)},${CHART_H} L0,${CHART_H} Z` };
});

const hint = computed(() => {
  if (r.value) return r.value.via_tunnel ? "Measured through the active tunnel." : "Measured on your direct connection (not connected).";
  return store.status.running ? "Runs through the active tunnel. Takes about 25 seconds." : "Not connected: this tests your direct connection. Takes about 25 seconds.";
});

const icons = { ok: CircleCheck, warn: TriangleAlert, fail: CircleX, info: Info } as const;
const tone = { ok: "text-accent-soft", warn: "text-yellow-300", fail: "text-red-400", info: "text-white/55" } as const;
const word = { ok: "OK", warn: "Warning", fail: "Problem", info: "Info" } as const;
</script>

<template>
  <Modal :title="t('Network tools')" wide @close="store.ui.tools = false">
    <div class="mb-4 flex gap-1 rounded-lg border border-ink-line bg-black/40 p-1" role="tablist">
      <button
        v-for="tb in tabs"
        :key="tb.id"
        role="tab"
        :aria-selected="tab === tb.id"
        class="flex-1 rounded-md px-3 py-1.5 text-xs transition"
        :class="tab === tb.id ? 'bg-accent/15 text-accent-soft' : 'text-white/50 hover:text-white/80'"
        @click="tab = tb.id"
      >
        {{ t(tb.label) }}
      </button>
    </div>

    <!-- speed test -->
    <div v-if="tab === 'speed'">
      <div class="mb-2 flex flex-wrap items-center justify-center gap-x-4 gap-y-1 text-[11px] text-white/55">
        <span>{{ r?.server ?? "Cloudflare" }}</span>
        <span>{{ (r ? r.via_tunnel : store.status.running) ? t("via tunnel") : t("direct connection") }}</span>
        <template v-if="r">
          <span v-if="r.isp">{{ r.isp }}</span>
          <span v-if="r.location">{{ r.location }}</span>
          <span v-if="r.ip" class="font-mono" style="user-select: text">{{ r.ip }}</span>
        </template>
      </div>

      <!-- idle: big GO like Speedtest.net -->
      <div v-if="idle" class="grid place-items-center py-6">
        <button
          class="grid h-40 w-40 place-items-center rounded-full border-2 border-accent/60 bg-accent/5 text-4xl font-semibold tracking-[0.25em] text-accent-soft shadow-[0_0_40px_rgba(34,197,94,.25)] transition hover:bg-accent/15 hover:shadow-[0_0_56px_rgba(34,197,94,.4)]"
          :aria-label="t('Start speed test')"
          @click="runSpeedTest"
        >
          {{ t("GO") }}
        </button>
      </div>

      <template v-else>
        <SpeedGauge :value="gaugeValue" :color="gaugeColor" :caption="t(caption)" />

        <div class="mt-1 grid grid-cols-3 gap-3 text-center">
          <div
            v-for="c in cards"
            :key="c.p"
            class="rounded-xl border bg-black/30 px-2 py-3 transition-colors"
            :class="c.active ? 'border-accent/60' : 'border-ink-line'"
          >
            <div class="hud-label">{{ t(c.label) }}</div>
            <div class="mt-1 font-mono text-xl text-white">{{ c.value }}</div>
            <div class="text-[10px] text-white/45">{{ c.unit }}<template v-if="c.p === 'ping' && r"> {{ t("· jitter") }} {{ r.jitter_ms.toFixed(0) }} {{ t("ms") }}</template></div>
          </div>
        </div>

        <svg v-if="sp.running && chart.line" :viewBox="`0 0 ${CHART_W} ${CHART_H}`" class="mt-3 h-14 w-full" preserveAspectRatio="none" aria-hidden="true">
          <path :d="chart.area" :fill="gaugeColor" fill-opacity="0.12" />
          <path :d="chart.line" fill="none" :stroke="gaugeColor" stroke-width="1.5" vector-effect="non-scaling-stroke" />
        </svg>
      </template>

      <p v-if="sp.error" class="mt-3 rounded-lg border border-red-500/40 p-2 text-xs text-red-300" role="alert">{{ sp.error }}</p>
      <ul v-if="r?.notes.length" class="mt-3 space-y-1">
        <li v-for="n in r.notes" :key="n" class="flex items-start gap-2 text-xs text-yellow-300" role="alert"><TriangleAlert :size="13" class="mt-0.5 shrink-0" /> {{ tDetail(n) }}</li>
      </ul>

      <div class="mt-3 flex items-center justify-between gap-3">
        <p class="text-xs text-white/50">{{ t(hint) }}</p>
        <button v-if="!idle" class="btn shrink-0" :disabled="sp.running" @click="runSpeedTest">
          <Play :size="13" /> {{ sp.running ? t("Testing…") : t("Run again") }}
        </button>
      </div>
    </div>

    <!-- leak test -->
    <div v-else>
      <p class="mb-3 text-xs text-white/50">{{ t("Compares your direct connection with the tunnel: exit IP, DNS resolver and IPv6.") }}</p>
      <ul v-if="store.diag.result" class="space-y-2">
        <li v-for="c in store.diag.result.checks" :key="c.label" class="flex items-start gap-3 rounded-xl border border-ink-line bg-black/30 px-3 py-2.5">
          <component :is="icons[c.status]" :size="16" class="mt-0.5 shrink-0" :class="tone[c.status]" />
          <div class="min-w-0">
            <div class="text-sm text-white/90">
              {{ t(c.label) }} <span class="ms-1 text-[10px] uppercase tracking-wider" :class="tone[c.status]">{{ t(word[c.status]) }}</span>
            </div>
            <div class="break-words text-xs text-white/55" style="user-select: text">{{ tDetail(c.detail) }}</div>
          </div>
        </li>
      </ul>
      <p v-if="store.diag.error" class="rounded-lg border border-red-500/40 p-2 text-xs text-red-300" role="alert">{{ store.diag.error }}</p>
      <button class="btn mt-3" :disabled="store.diag.running" @click="runDiagnostics">
        <Play :size="13" /> {{ store.diag.running ? t("Checking…") : store.diag.result ? t("Run again") : t("Run test") }}
      </button>
    </div>
  </Modal>
</template>
