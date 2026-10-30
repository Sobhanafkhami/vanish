<script setup lang="ts">
import { t } from "./lib/i18n";
import { computed, onMounted, ref } from "vue";
import { ChevronDown, Home, List, ChevronRight, Download, Globe, Rocket, RefreshCw, ShieldAlert, TriangleAlert, X } from "lucide-vue-next";
import TitleBar from "./components/TitleBar.vue";
import ConnectButton from "./components/ConnectButton.vue";
import TrafficMonitor from "./components/TrafficMonitor.vue";
import GameMode from "./components/GameMode.vue";
import { isMobile } from "./lib/platform";
import NodeList from "./components/NodeList.vue";
import SettingsModal from "./components/SettingsModal.vue";
import LogsModal from "./components/LogsModal.vue";
import QrModal from "./components/QrModal.vue";
import ToolsModal from "./components/ToolsModal.vue";
import AppPickerModal from "./components/AppPickerModal.vue";
import { clearError, init, installUpdate, measureLatency, setMode, store } from "./stores/app";

onMounted(init);

const modes = [
  { id: "system_proxy", label: "System Proxy", hint: "Browsers and apps that follow the Windows proxy settings." },
  { id: "tun", label: "TUN", hint: "Captures all traffic on this PC. Needs administrator rights." },
  { id: "manual", label: "Manual", hint: "Only apps you point at the local port." },
] as const;

// Game mode always runs on TUN (games use UDP and ignore the system proxy), so the selector shows and locks that.
const gameOn = computed(() => !!store.settings?.game_mode);
const activeMode = computed(() => (gameOn.value ? "tun" : store.settings?.mode));
const hints = [
  ...modes.map((m) => ({ key: m.id as string, text: m.hint as string })),
  { key: "game", text: "Game mode uses TUN." },
  { key: "running", text: "Disconnect to change the mode." },
];
const hintKey = computed(() => (gameOn.value ? "game" : store.status.running ? "running" : (activeMode.value ?? "system_proxy")));
const modeLocked = computed(() => store.status.running || gameOn.value);

// Phones show one column at a time (bottom tabs); desktop shows both side by side.
const view = ref<"home" | "servers">("home");
const showDetails = ref(false);
const updateDismissed = ref(false);
const showRaw = ref(false);

const latencyText = computed(() => (store.latency === null ? "—" : String(store.latency)));
const latencyTone = computed(() => {
  const ms = store.latency;
  if (ms === null) return "text-white/40";
  return ms < 300 ? "text-accent-soft" : ms < 800 ? "text-yellow-300" : "text-red-400";
});

const location = computed(() => {
  const e = store.exit;
  if (!e) return "";
  return [e.city, e.region, e.country].filter(Boolean).join(", ");
});

function dismissError() {
  showRaw.value = false;
  clearError();
}
</script>

<template>
  <div class="relative flex h-full flex-col overflow-hidden bg-ink">
    <!-- slow drifting glow behind everything; turns greener while connected -->
    <div class="pointer-events-none absolute inset-0 overflow-hidden" aria-hidden="true">
      <span class="ambient ambient-a" :class="{ 'is-on': store.status.running }"></span>
      <span class="ambient ambient-b" :class="{ 'is-on': store.status.running }"></span>
    </div>
    <TitleBar />

    <Transition name="fade">
      <!-- a bar under the title on desktop, a snackbar above the tabs on phones -->
      <div
        v-if="store.update && !updateDismissed"
        class="flex items-center justify-between gap-3 border-b border-accent/20 bg-accent/10 px-4 py-2 text-xs max-md:fixed max-md:inset-x-3 max-md:bottom-[4.75rem] max-md:z-40 max-md:rounded-xl max-md:border max-md:border-accent/40 max-md:bg-ink-raised max-md:px-3 max-md:py-2.5 max-md:text-sm max-md:shadow-xl"
        role="status"
      >
        <span class="flex items-center gap-2 text-accent-soft"><Download :size="14" /> {{ t("Version {v} is available", { v: store.update.version }) }}</span>
        <span class="flex shrink-0 items-center gap-1">
          <button class="btn !border-accent/50 !text-accent-soft" :disabled="store.updating" @click="installUpdate">{{ store.updating ? t("Updating…") : t("Update now") }}</button>
          <button class="grid h-9 w-9 place-items-center rounded-lg text-white/50 hover:text-white md:hidden" :aria-label="t('Dismiss')" @click="updateDismissed = true"><X :size="16" /></button>
        </span>
      </div>
    </Transition>

    <main class="relative grid min-h-0 flex-1 grid-cols-1 gap-4 p-4 md:grid-cols-[320px_1fr]">
      <!-- left: the connection itself -->
      <section class="-me-2 min-h-0 flex-col gap-3 overflow-y-auto pe-2 md:flex" :class="view === 'home' ? 'flex' : 'hidden'">
        <div class="panel rise flex shrink-0 flex-col items-center gap-4 px-4 py-5" :class="{ active: store.status.running }" style="--d: 0ms">
          <ConnectButton />

          <!-- Android has one way in (the VPN service), so there is no mode to pick there -->
          <div v-if="!isMobile" class="w-full">
            <div class="grid grid-cols-[1.5fr_1fr_1fr] gap-1 rounded-lg border border-ink-line bg-black/40 p-1" role="radiogroup" :aria-label="t('Mode')">
              <button
                v-for="m in modes"
                :key="m.id"
                role="radio"
                :aria-checked="activeMode === m.id"
                :disabled="modeLocked"
                :title="gameOn ? t('Game mode uses TUN.') : store.status.running ? t('Disconnect to change the mode.') : t(m.hint)"
                class="h-8 whitespace-nowrap rounded-md px-1 text-xs transition disabled:cursor-not-allowed"
                :class="activeMode === m.id ? 'bg-accent/15 font-semibold text-accent-soft' : 'text-white/55 hover:text-white/85 disabled:hover:text-white/55'"
                @click="setMode(m.id)"
              >
                {{ t(m.label) }}
              </button>
            </div>
            <!-- every hint sits in the same grid cell, so the box is always as tall as the longest one and never jumps -->
            <div class="mt-2 grid text-center text-[11px] leading-snug text-white/45" aria-live="polite">
              <p
                v-for="h in hints"
                :key="h.key"
                class="col-start-1 row-start-1 transition-opacity duration-200"
                :class="h.key === hintKey ? 'opacity-100' : 'pointer-events-none opacity-0'"
                :aria-hidden="h.key !== hintKey"
              >
                {{ t(h.text) }}
              </p>
            </div>
          </div>
        </div>

        <!-- where you appear to be -->
        <div class="panel rise shrink-0 px-4 py-3" style="--d: 80ms">
          <div class="flex items-center justify-between">
            <span class="flex items-center gap-2">
              <Globe :size="15" class="text-accent-soft" />
              <span class="hud-label">{{ t("Connection") }}</span>
            </span>
            <span class="flex items-center gap-1.5">
              <span class="font-mono text-sm" :class="latencyTone">{{ latencyText }}<span class="ms-0.5 text-[10px] text-white/45">{{ t("ms") }}</span></span>
              <button
                class="rounded p-1 text-white/40 transition hover:text-accent-soft disabled:opacity-30"
                :disabled="!store.status.running"
                :aria-label="t('Measure latency')"
                :title="t('Measure latency')"
                @click="measureLatency"
              >
                <RefreshCw :size="13" />
              </button>
            </span>
          </div>

          <template v-if="store.status.running && store.exit">
            <div class="mt-3 flex items-center justify-between gap-2" style="user-select: text">
              <span class="min-w-0 truncate font-mono text-lg text-white" dir="ltr">{{ store.exit.ip }}</span>
              <span class="shrink-0 rounded border border-accent/40 px-1.5 py-0.5 font-mono text-[10px] text-accent-soft">{{ store.exit.country_code }}</span>
            </div>
            <div class="mt-1 truncate text-sm text-white/85">{{ location || "—" }}</div>
            <div class="truncate text-xs text-white/50" :title="store.exit.isp">{{ store.exit.isp || "—" }}</div>

            <button class="mt-2 flex items-center gap-1 text-[11px] text-white/45 transition hover:text-white/80" :aria-expanded="showDetails" @click="showDetails = !showDetails">
              <ChevronDown :size="12" class="transition" :class="{ 'rotate-180': showDetails }" />
              {{ showDetails ? t("Hide details") : t("Show details") }}
            </button>
            <dl v-if="showDetails" class="mt-2 grid grid-cols-[4rem_1fr] gap-x-2 gap-y-1 text-xs" style="user-select: text">
              <dt class="text-white/45">{{ t("ZIP") }}</dt>
              <dd class="font-mono text-white/85">{{ store.exit.zip || "—" }}</dd>
              <dt class="text-white/45">{{ t("Host") }}</dt>
              <dd class="truncate font-mono text-white/85" dir="ltr" :title="store.exit.host">{{ store.exit.host || "—" }}</dd>
            </dl>
          </template>
          <p v-else class="mt-3 text-xs leading-relaxed text-white/45">{{ t("Connect to see your new location.") }}</p>
        </div>

        <div class="rise" style="--d: 160ms"><GameMode /></div>
        <div class="rise" style="--d: 240ms"><TrafficMonitor /></div>

        <button class="panel rise flex shrink-0 items-center justify-between px-4 py-3 text-start transition hover:border-accent/40" style="--d: 320ms" @click="store.ui.tools = true">
          <span class="flex items-center gap-2">
            <Rocket :size="15" class="text-accent-soft" />
            <span class="text-xs text-white/80">{{ t("Speed & leak test") }}</span>
          </span>
          <span v-if="store.speed.result" class="font-mono text-xs text-white/60">↓ {{ store.speed.result.download_mbps.toFixed(0) }} ↑ {{ store.speed.result.upload_mbps.toFixed(0) }}</span>
          <ChevronRight v-else :size="14" class="text-white/40 rtl:rotate-180" />
        </button>
      </section>

      <!-- right: servers -->
      <section class="rise min-h-0 flex-col md:flex" :class="view === 'servers' ? 'flex' : 'hidden'" style="--d: 120ms">
        <NodeList />
      </section>
    </main>

    <!-- bottom navigation, phones only -->
    <nav class="flex shrink-0 border-t border-ink-line bg-black/50 md:hidden" :style="{ paddingBottom: 'env(safe-area-inset-bottom)' }" role="tablist">
      <button
        v-for="tab in ([{ id: 'home', label: 'Connection', icon: Home }, { id: 'servers', label: 'Servers', icon: List }] as const)"
        :key="tab.id"
        role="tab"
        :aria-selected="view === tab.id"
        class="flex min-h-[3.25rem] flex-1 flex-col items-center justify-center gap-0.5 text-xs transition"
        :class="view === tab.id ? 'text-accent-soft' : 'text-white/50'"
        @click="view = tab.id"
      >
        <component :is="tab.icon" :size="20" />
        {{ t(tab.label) }}
      </button>
    </nav>

    <!-- messages float over the content so they never push the layout around -->
    <div class="pointer-events-none absolute inset-x-4 bottom-20 z-30 md:bottom-4 flex flex-col items-end gap-2">
      <Transition name="fade">
        <div v-if="store.status.blocked" class="pointer-events-auto flex w-full max-w-md items-start gap-2 rounded-xl border border-red-500/40 bg-ink-raised p-3 text-xs text-red-300 shadow-lg" role="alert">
          <ShieldAlert :size="16" class="mt-0.5 shrink-0" />
          <span>{{ t("The core stopped unexpectedly. The kill switch is blocking traffic. Press the power button to reconnect.") }}</span>
        </div>
      </Transition>
      <Transition name="fade">
        <div v-if="store.notice" class="pointer-events-auto w-full max-w-md rounded-xl border border-accent/30 bg-ink-raised p-3 text-xs text-accent-soft shadow-lg" role="status">{{ store.notice }}</div>
      </Transition>
      <Transition name="fade">
        <div v-if="store.error" class="pointer-events-auto w-full max-w-md rounded-xl border border-red-500/40 bg-ink-raised p-3 text-xs shadow-lg" role="alert">
          <div class="flex items-start gap-2">
            <TriangleAlert :size="16" class="mt-0.5 shrink-0 text-red-400" />
            <div class="min-w-0 flex-1">
              <div class="text-sm font-semibold text-red-300">{{ store.error }}</div>
              <p class="mt-0.5 leading-relaxed text-white/70">{{ store.errorHint }}</p>
              <button v-if="store.errorRaw" class="mt-1.5 text-[11px] text-white/40 transition hover:text-white/75" @click="showRaw = !showRaw">
                {{ showRaw ? t("Hide details") : t("Technical details") }}
              </button>
              <pre v-if="showRaw" dir="ltr" class="mt-1.5 max-h-32 overflow-auto whitespace-pre-wrap break-words rounded-lg bg-black/40 p-2 text-left font-mono text-[10px] text-white/55" style="user-select: text">{{ store.errorRaw }}</pre>
            </div>
            <button class="shrink-0 rounded p-1 text-white/40 transition hover:text-white" :aria-label="t('Dismiss')" @click="dismissError"><X :size="14" /></button>
          </div>
        </div>
      </Transition>
    </div>

    <SettingsModal v-if="store.ui.settings" />
    <LogsModal v-if="store.ui.logs" />
    <QrModal v-if="store.ui.qr" />
    <ToolsModal v-if="store.ui.tools" />
    <AppPickerModal v-if="store.ui.appPicker" />
  </div>
</template>
