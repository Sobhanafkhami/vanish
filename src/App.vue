<script setup lang="ts">
import { computed, onMounted } from "vue";
import { Gauge, RefreshCw } from "lucide-vue-next";
import TitleBar from "./components/TitleBar.vue";
import ConnectButton from "./components/ConnectButton.vue";
import TrafficMonitor from "./components/TrafficMonitor.vue";
import NodeList from "./components/NodeList.vue";
import { init, measureLatency, setMode, store } from "./stores/app";

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
    class="flex h-full flex-col overflow-hidden rounded-xl border border-ink-line bg-ink/95"
  >
    <TitleBar />
    <main class="grid min-h-0 flex-1 grid-cols-[320px_1fr] gap-4 p-4">
      <section class="flex flex-col gap-4">
        <div class="panel flex flex-col items-center gap-5 py-6" :class="{ active: store.status.running }">
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
              {{ m.label }}
            </button>
          </div>
        </div>

        <div class="panel flex items-center justify-between px-4 py-3">
          <div class="flex items-center gap-2">
            <Gauge :size="16" class="text-accent-soft" />
            <span class="hud-label">Latency</span>
          </div>
          <div class="flex items-center gap-3">
            <span class="font-mono text-2xl text-white">{{ latencyText }}<span class="ml-1 text-xs text-white/50">ms</span></span>
            <button class="text-white/40 transition hover:text-accent-soft disabled:opacity-30" :disabled="!store.status.running" @click="measureLatency">
              <RefreshCw :size="14" />
            </button>
          </div>
        </div>

        <TrafficMonitor />
      </section>

      <section class="flex min-h-0 flex-col gap-3">
        <NodeList />
        <Transition name="fade">
          <div v-if="store.error" class="panel whitespace-pre-wrap border-red-500/40 p-3 font-mono text-xs text-red-300">{{ store.error }}</div>
        </Transition>
      </section>
    </main>
  </div>
</template>
