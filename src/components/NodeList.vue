<script setup lang="ts">
import { ref } from "vue";
import { Activity, ClipboardPaste, Trash2 } from "lucide-vue-next";
import { importInput, pingAll, removeNode, selectNode, store } from "../stores/app";

const input = ref("");
const showImport = ref(false);
const message = ref("");

async function doImport() {
  if (!input.value.trim()) return;
  message.value = await importInput(input.value);
  input.value = "";
  showImport.value = false;
  setTimeout(() => (message.value = ""), 4000);
}

function latencyClass(ms: number | null) {
  if (ms === null) return "text-white/30";
  return ms < 150 ? "text-emerald-400" : ms < 400 ? "text-yellow-300" : "text-red-400";
}
</script>

<template>
  <div class="glass flex min-h-0 flex-1 flex-col p-4">
    <div class="mb-3 flex items-center justify-between">
      <span class="hud-label">Nodes · {{ store.nodes.length }}</span>
      <div class="flex gap-2">
        <button class="btn" :disabled="store.busy || !store.nodes.length" @click="pingAll"><Activity :size="13" /> TCP ping</button>
        <button class="btn" @click="showImport = !showImport"><ClipboardPaste :size="13" /> Import</button>
      </div>
    </div>

    <Transition name="fade">
      <div v-if="showImport" class="mb-3">
        <textarea
          v-model="input"
          rows="3"
          placeholder="Subscription URL, vless:// vmess:// trojan:// ss:// wireguard:// links, or sing-box outbound JSON"
          class="w-full resize-none rounded-lg border border-white/10 bg-black/40 p-2 font-mono text-xs text-white outline-none placeholder:text-white/25 focus:border-cyan-neon/60"
        />
        <button class="btn mt-2" :disabled="store.busy" @click="doImport">Add</button>
      </div>
    </Transition>
    <div v-if="message" class="mb-2 font-mono text-xs text-cyan-neon">{{ message }}</div>

    <div class="-mr-2 flex-1 space-y-2 overflow-y-auto pr-2">
      <div v-if="!store.nodes.length" class="grid h-full place-items-center text-center text-xs text-white/30">
        No nodes yet.<br />Import a subscription or paste a config link.
      </div>
      <div
        v-for="n in store.nodes"
        :key="n.id"
        class="group flex cursor-pointer items-center gap-3 rounded-xl border px-3 py-2 transition"
        :class="
          store.settings?.selected_node === n.id
            ? 'border-cyan-neon/60 bg-cyan-neon/5 shadow-glow-cyan'
            : 'border-white/5 bg-white/[0.03] hover:border-white/20'
        "
        @click="selectNode(n.id)"
      >
        <span class="rounded border border-violet-neon/50 px-1.5 py-0.5 font-mono text-[9px] uppercase text-violet-300">{{ n.protocol }}</span>
        <div class="min-w-0 flex-1">
          <div class="truncate text-sm text-white/90">{{ n.name }}</div>
          <div class="truncate font-mono text-[10px] text-white/35">{{ n.server }}:{{ n.port }}</div>
        </div>
        <span class="w-14 text-right font-mono text-xs" :class="latencyClass(n.latency)">{{ n.latency === null ? "—" : n.latency + " ms" }}</span>
        <button class="text-white/20 opacity-0 transition hover:text-red-400 group-hover:opacity-100" @click.stop="removeNode(n.id)">
          <Trash2 :size="14" />
        </button>
      </div>
    </div>
  </div>
</template>
