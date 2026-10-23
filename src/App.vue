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
    class="flex h-full flex-col overflow-hidden rounded-xl border border-white/10 bg-obsidian/85"
    style="background-image: radial-gradient(circle at 15% 0%, rgba(112, 0, 255, 0.18), transparent 45%), radial-gradient(circle at 100% 100%, rgba(0, 242, 254, 0.12), transparent 40%)"
  >
    <TitleBar />
    <main class="grid min-h-0 flex-1 grid-cols-[320px_1fr] gap-4 p-4">
      <section class="flex flex-col gap-4">
        <div class="glass flex flex-col items-center gap-5 py-6" :class="{ active: store.status.running }">
          <ConnectButton />

          <div class="flex gap-1 rounded-lg border border-white/10 bg-black/30 p-1">
            <button
              v-for="m in modes"
              :key="m.id"
              :disabled="store.status.running"
              class="rounded-md px-3 py-1 font-mono text-[10px] uppercase tracking-wider transition disabled:cursor-not-allowed"
              :class="store.settings?.mode === m.id ? 'bg-cyan-neon/15 text-cyan-neon' : 'text-white/40 hover:text-white/80'"
              @click="setMode(m.id)"
            >
              {{ m.label }}
            </button>
          </div>
        </div>

        <div class="glass flex items-center justify-between px-4 py-3">
          <div class="flex items-center gap-2">
            <Gauge :size="16" class="text-violet-400" />
            <span class="hud-label">Latency</span>
          </div>
          <div class="flex items-center gap-3">
            <span class="font-mono text-2xl text-white">{{ latencyText }}<span class="ml-1 text-xs text-white/40">ms</span></span>
            <button class="text-white/40 transition hover:text-cyan-neon disabled:opacity-30" :disabled="!store.status.running" @click="measureLatency">
              <RefreshCw :size="14" />
            </button>
          </div>
        </div>

        <TrafficMonitor />
      </section>

      <section class="flex min-h-0 flex-col gap-3">
        <NodeList />
        <Transition name="fade">
          <div v-if="store.error" class="glass whitespace-pre-wrap border-red-500/40 p-3 font-mono text-xs text-red-300">{{ store.error }}</div>
        </Transition>
      </section>
    </main>
  </div>
</template>
