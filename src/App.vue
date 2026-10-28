<script setup lang="ts">
import { t } from "./lib/i18n";
import { computed, onMounted } from "vue";
import { ChevronRight, Download, Gauge, Globe, RefreshCw, Rocket, ShieldAlert } from "lucide-vue-next";
import TitleBar from "./components/TitleBar.vue";
import ConnectButton from "./components/ConnectButton.vue";
import TrafficMonitor from "./components/TrafficMonitor.vue";
import NodeList from "./components/NodeList.vue";
import SettingsModal from "./components/SettingsModal.vue";
import LogsModal from "./components/LogsModal.vue";
import QrModal from "./components/QrModal.vue";
import ToolsModal from "./components/ToolsModal.vue";
import AppPickerModal from "./components/AppPickerModal.vue";
import { init, installUpdate, measureLatency, setMode, store } from "./stores/app";

onMounted(init);

const modes = [
  { id: "system_proxy", label: "System Proxy" },
  { id: "tun", label: "TUN" },
  { id: "manual", label: "Manual" },
] as const;

const latencyText = computed(() => (store.latency === null ? "---" : String(store.latency)));
</script>

<template>
  <div
    class="relative flex h-full flex-col overflow-hidden bg-ink"
  >
    <TitleBar />
    <main class="grid min-h-0 flex-1 grid-cols-[320px_1fr] gap-4 p-4">
      <section class="-me-2 flex min-h-0 flex-col gap-4 overflow-y-auto pe-2">
        <div class="panel flex shrink-0 flex-col items-center gap-5 py-6" :class="{ active: store.status.running }">
          <ConnectButton />

          <div class="flex gap-1 rounded-lg border border-ink-line bg-black/40 p-1">
            <button
              v-for="m in modes"
              :key="m.id"
              :disabled="store.status.running"
              class="rounded-md px-3 py-1 font-mono text-[10px] uppercase tracking-wider transition disabled:cursor-not-allowed"
              :class="store.settings?.mode === m.id ? 'bg-accent/15 text-accent-soft' : 'text-white/50 hover:text-white/80'"
              @click="setMode(m.id)"
            >
              {{ t(m.label) }}
            </button>
          </div>
        </div>

        <div class="panel flex shrink-0 items-center justify-between px-4 py-3">
          <div class="flex items-center gap-2">
            <Gauge :size="16" class="text-accent-soft" />
            <span class="hud-label">{{ t("Latency") }}</span>
          </div>
          <div class="flex items-center gap-3">
            <span class="font-mono text-2xl text-white">{{ latencyText }}<span class="ms-1 text-xs text-white/50">{{ t("ms") }}</span></span>
            <button class="text-white/40 transition hover:text-accent-soft disabled:opacity-30" :disabled="!store.status.running" @click="measureLatency">
              <RefreshCw :size="14" />
            </button>
          </div>
        </div>

        <div v-if="store.exit" class="panel shrink-0 px-4 py-3">
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-2">
              <Globe :size="16" class="text-accent-soft" />
              <span class="hud-label">{{ t("Exit") }}</span>
            </div>
            <span class="font-mono text-sm text-white" style="user-select: text">
              {{ store.exit.ip }}
              <span class="ms-1 rounded border border-accent/40 px-1 py-0.5 text-[10px] text-accent-soft">{{ store.exit.country_code }}</span>
            </span>
          </div>
          <dl class="mt-2 grid grid-cols-[3.5rem_1fr] gap-x-2 gap-y-1 text-xs" style="user-select: text">
            <dt class="text-white/45">{{ t("Country") }}</dt>
            <dd class="truncate text-white/85">{{ store.exit.country }}</dd>
            <dt class="text-white/45">{{ t("City") }}</dt>
            <dd class="truncate text-white/85">{{ [store.exit.city, store.exit.region].filter(Boolean).join(", ") || "—" }}</dd>
            <dt class="text-white/45">{{ t("ZIP") }}</dt>
            <dd class="font-mono text-white/85">{{ store.exit.zip || "—" }}</dd>
            <dt class="text-white/45">{{ t("Host") }}</dt>
            <dd class="truncate font-mono text-white/85" :title="store.exit.host">{{ store.exit.host || "—" }}</dd>
            <dt class="text-white/45">{{ t("ISP") }}</dt>
            <dd class="truncate text-white/85" :title="store.exit.isp">{{ store.exit.isp || "—" }}</dd>
          </dl>
        </div>

        <button class="panel flex shrink-0 items-center justify-between px-4 py-3 text-start hover:border-accent/40" @click="store.ui.tools = true">
          <span class="flex items-center gap-2">
            <Rocket :size="16" class="text-accent-soft" />
            <span class="hud-label">{{ t("Speed & leak test") }}</span>
          </span>
          <span v-if="store.speed.result" class="font-mono text-xs text-white/60">↓ {{ store.speed.result.download_mbps.toFixed(0) }} ↑ {{ store.speed.result.upload_mbps.toFixed(0) }}</span>
          <ChevronRight v-else :size="14" class="text-white/40 rtl:rotate-180" />
        </button>

        <TrafficMonitor />
      </section>

      <section class="flex min-h-0 flex-col gap-3">
        <NodeList />
        <Transition name="fade">
          <div v-if="store.update" class="panel flex items-center justify-between gap-3 border-accent/30 p-3 text-xs" role="status">
            <span class="flex items-center gap-2 text-accent-soft"><Download :size="14" /> {{ t("Version {v} is available", { v: store.update.version }) }}</span>
            <button class="btn !border-accent/50 !text-accent-soft" :disabled="store.updating" @click="installUpdate">{{ store.updating ? t("Updating…") : t("Update now") }}</button>
          </div>
        </Transition>
        <Transition name="fade">
          <div v-if="store.status.blocked" class="panel flex items-start gap-2 border-red-500/40 p-3 text-xs text-red-300" role="alert">
            <ShieldAlert :size="16" class="mt-0.5 shrink-0" />
            <span>{{ t("The core stopped unexpectedly. The kill switch is blocking traffic. Press the power button to reconnect.") }}</span>
          </div>
        </Transition>
        <Transition name="fade">
          <div v-if="store.notice" class="panel border-accent/30 p-3 text-xs text-accent-soft" role="status">{{ store.notice }}</div>
        </Transition>
        <Transition name="fade">
          <div v-if="store.error" class="panel whitespace-pre-wrap border-red-500/40 p-3 font-mono text-xs text-red-300">{{ store.error }}</div>
        </Transition>
      </section>
    </main>
    <SettingsModal v-if="store.ui.settings" />
    <LogsModal v-if="store.ui.logs" />
    <QrModal v-if="store.ui.qr" />
    <ToolsModal v-if="store.ui.tools" />
    <AppPickerModal v-if="store.ui.appPicker" />
  </div>
</template>
