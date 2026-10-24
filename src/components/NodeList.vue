<script setup lang="ts">
import { computed, nextTick, ref } from "vue";
import { Activity, ArrowDown, ArrowDownUp, ArrowUp, Check, ClipboardPaste, Pencil, QrCode, RefreshCw, Search, Timer, Trash2, X, Zap } from "lucide-vue-next";
import SubUsage from "./SubUsage.vue";
import { fmtBytes, subUsage, usageTone } from "../lib/usage";
import {
  addSubscription,
  connectFastest,
  importLinks,
  pingAll,
  realDelay,
  refreshSubscription,
  removeNode,
  removeSubscription,
  renameSubscription,
  selectNode,
  store,
} from "../stores/app";

const input = ref("");
const subName = ref("");
const subUrl = ref("");
const importKind = ref<"sub" | "links">("sub");
const showImport = ref(false);
const message = ref("");
const query = ref("");
const sortByPing = ref(false);
const renaming = ref(false);
const renameValue = ref("");
const renameInput = ref<HTMLInputElement | null>(null);

function flash(text: string) {
  message.value = text;
  setTimeout(() => (message.value = ""), 4000);
}

async function doImport() {
  if (importKind.value === "sub") {
    if (!subUrl.value.trim()) return;
    const res = await addSubscription(subName.value, subUrl.value);
    flash(res);
    if (!store.error) {
      subName.value = subUrl.value = "";
      showImport.value = false;
    }
  } else {
    if (!input.value.trim()) return;
    flash(await importLinks(input.value));
    input.value = "";
    showImport.value = false;
  }
}

async function paste() {
  try {
    const text = (await navigator.clipboard.readText()).trim();
    if (!text) return;
    if (/^https?:\/\//i.test(text) && !text.includes("\n")) {
      importKind.value = "sub";
      subUrl.value = text;
    } else {
      importKind.value = "links";
      input.value = text;
    }
  } catch {
    flash("Clipboard access was denied");
  }
}

const manualCount = computed(() => store.nodes.filter((n) => !n.subscription).length);
const activeSub = computed(() => store.subs.find((s) => s.id === store.activeTab) ?? null);

const visible = computed(() => {
  const tab = store.activeTab;
  const q = query.value.trim().toLowerCase();
  let list = store.nodes.filter((n) => (tab === "all" ? true : tab === "manual" ? !n.subscription : n.subscription === tab));
  if (q) list = list.filter((n) => `${n.name} ${n.server} ${n.protocol}`.toLowerCase().includes(q));
  if (sortByPing.value) {
    // Prefer the real HTTP delay when there is one, fall back to the TCP ping.
    const key = (n: (typeof list)[number]) => (n.delay !== null && n.delay > 0 ? n.delay : n.delay === -1 ? Infinity : (n.latency ?? Infinity));
    list = [...list].sort((a, b) => key(a) - key(b));
  }
  return list;
});

function countFor(subId: string) {
  return store.nodes.filter((n) => n.subscription === subId).length;
}

function latencyClass(ms: number | null, good = 150, ok = 400) {
  if (ms === null) return "text-white/40";
  return ms < good ? "text-accent-soft" : ms < ok ? "text-yellow-300" : "text-red-400";
}

function delayText(d: number | null) {
  if (d === null) return store.delayTesting ? "…" : "—";
  return d < 0 ? "timeout" : `${d} ms`;
}

const updatedText = computed(() => {
  const t = activeSub.value?.updated_at;
  return t ? new Date(t * 1000).toLocaleString() : "never";
});

async function startRename() {
  if (!activeSub.value) return;
  renameValue.value = activeSub.value.name;
  renaming.value = true;
  await nextTick();
  renameInput.value?.focus();
}
async function commitRename() {
  if (activeSub.value && renameValue.value.trim()) await renameSubscription(activeSub.value.id, renameValue.value);
  renaming.value = false;
}
async function deleteSub() {
  if (!activeSub.value) return;
  if (confirm(`Remove "${activeSub.value.name}" and its ${countFor(activeSub.value.id)} node(s)?`)) await removeSubscription(activeSub.value.id);
}
</script>

<template>
  <div class="panel flex min-h-0 flex-1 flex-col p-4">
    <div class="mb-3 flex items-center justify-between">
      <span class="hud-label">Nodes · {{ visible.length }}</span>
      <div class="flex gap-2">
        <button class="btn" :disabled="store.busy || !store.nodes.length" title="Ping all, then select the quickest" @click="connectFastest">
          <Zap :size="13" /> Fastest
        </button>
        <button class="btn" :disabled="store.busy || !store.nodes.length" title="TCP connect time to each server" @click="pingAll"><Activity :size="13" /> TCP ping</button>
        <button
          class="btn"
          :disabled="store.delayTesting || !visible.length"
          title="Real HTTP request through each node, like URL test"
          @click="realDelay(visible.map((n) => n.id))"
        >
          <Timer :size="13" /> {{ store.delayTesting ? "Testing…" : "Real delay" }}
        </button>
        <button class="btn" @click="showImport = !showImport"><ClipboardPaste :size="13" /> Import</button>
      </div>
    </div>

    <Transition name="fade">
      <div v-if="showImport" class="mb-3 rounded-xl border border-ink-line bg-black/30 p-3">
        <div class="mb-3 flex items-center justify-between">
          <div class="flex gap-1 rounded-lg border border-ink-line bg-black/40 p-1">
            <button
              v-for="k in [{ id: 'sub', label: 'Subscription' }, { id: 'links', label: 'Links / JSON' }] as const"
              :key="k.id"
              class="rounded-md px-3 py-1 text-xs transition"
              :class="importKind === k.id ? 'bg-accent/15 text-accent-soft' : 'text-white/50 hover:text-white/80'"
              @click="importKind = k.id"
            >
              {{ k.label }}
            </button>
          </div>
          <button class="btn" @click="paste"><ClipboardPaste :size="13" /> Paste</button>
        </div>

        <div v-if="importKind === 'sub'" class="space-y-2">
          <input
            v-model="subName"
            maxlength="40"
            placeholder="Name (e.g. Work, Home)"
            class="w-full rounded-lg border border-ink-line bg-black/40 p-2 text-xs text-white outline-none placeholder:text-white/30 focus:border-accent/60"
          />
          <input
            v-model="subUrl"
            placeholder="https://example.com/sub/..."
            class="w-full rounded-lg border border-ink-line bg-black/40 p-2 font-mono text-xs text-white outline-none placeholder:text-white/30 focus:border-accent/60"
            @keydown.enter="doImport"
          />
        </div>
        <textarea
          v-else
          v-model="input"
          rows="3"
          placeholder="vless:// vmess:// trojan:// ss:// wireguard:// links, or sing-box outbound JSON"
          class="w-full resize-none rounded-lg border border-ink-line bg-black/40 p-2 font-mono text-xs text-white outline-none placeholder:text-white/30 focus:border-accent/60"
        />
        <button class="btn mt-2" :disabled="store.busy" @click="doImport">{{ importKind === "sub" ? "Add subscription" : "Add" }}</button>
      </div>
    </Transition>
    <div v-if="message" class="mb-2 font-mono text-xs text-accent-soft">{{ message }}</div>

    <!-- tabs: one per subscription -->
    <div v-if="store.subs.length || manualCount" class="mb-3 flex gap-1 overflow-x-auto border-b border-ink-line pb-2" role="tablist">
      <button
        role="tab"
        :aria-selected="store.activeTab === 'all'"
        class="shrink-0 rounded-md px-3 py-1 text-xs transition"
        :class="store.activeTab === 'all' ? 'bg-accent/15 text-accent-soft' : 'text-white/50 hover:text-white/80'"
        @click="store.activeTab = 'all'"
      >
        All <span class="ml-1 font-mono text-[10px] opacity-70">{{ store.nodes.length }}</span>
      </button>
      <button
        v-if="manualCount"
        role="tab"
        :aria-selected="store.activeTab === 'manual'"
        class="shrink-0 rounded-md px-3 py-1 text-xs transition"
        :class="store.activeTab === 'manual' ? 'bg-accent/15 text-accent-soft' : 'text-white/50 hover:text-white/80'"
        @click="store.activeTab = 'manual'"
      >
        Manual <span class="ml-1 font-mono text-[10px] opacity-70">{{ manualCount }}</span>
      </button>
      <button
        v-for="s in store.subs"
        :key="s.id"
        role="tab"
        :aria-selected="store.activeTab === s.id"
        class="max-w-[10rem] shrink-0 truncate rounded-md px-3 py-1 text-xs transition"
        :class="store.activeTab === s.id ? 'bg-accent/15 text-accent-soft' : 'text-white/50 hover:text-white/80'"
        @click="store.activeTab = s.id"
      >
        {{ s.name }} <span class="ml-1 font-mono text-[10px] opacity-70">{{ countFor(s.id) }}</span>
        <!-- remaining share at a glance -->
        <span v-if="subUsage(s).remainingPct !== null" class="mt-1 block h-0.5 overflow-hidden rounded-full bg-white/10" aria-hidden="true">
          <span class="block h-full rounded-full" :class="usageTone(subUsage(s).remainingPct)" :style="{ width: subUsage(s).remainingPct + '%' }"></span>
        </span>
      </button>
    </div>

    <!-- subscription details -->
    <div v-if="activeSub" class="mb-3 rounded-xl border border-ink-line bg-black/30 p-3">
      <div class="flex items-center justify-between gap-2">
        <div v-if="!renaming" class="min-w-0 truncate text-sm font-medium text-white">{{ activeSub.name }}</div>
        <input
          v-else
          ref="renameInput"
          v-model="renameValue"
          maxlength="40"
          class="min-w-0 flex-1 rounded border border-accent/60 bg-black/40 px-2 py-0.5 text-sm text-white outline-none"
          @keydown.enter="commitRename"
          @keydown.esc="renaming = false"
        />
        <div class="flex shrink-0 gap-1 text-white/50">
          <template v-if="renaming">
            <button class="p-1 hover:text-accent-soft" aria-label="Save name" @click="commitRename"><Check :size="14" /></button>
            <button class="p-1 hover:text-white" aria-label="Cancel" @click="renaming = false"><X :size="14" /></button>
          </template>
          <template v-else>
            <button class="p-1 hover:text-white" aria-label="Rename subscription" @click="startRename"><Pencil :size="14" /></button>
            <button class="p-1 hover:text-white" aria-label="Show subscription QR" @click="store.ui.qr = { title: activeSub.name, text: activeSub.url }">
              <QrCode :size="14" />
            </button>
            <button class="p-1 hover:text-accent-soft disabled:opacity-40" :disabled="store.busy" aria-label="Refresh subscription" @click="refreshSubscription(activeSub.id)">
              <RefreshCw :size="14" />
            </button>
            <button class="p-1 hover:text-red-400" aria-label="Remove subscription" @click="deleteSub"><Trash2 :size="14" /></button>
          </template>
        </div>
      </div>
      <div class="mt-1 truncate font-mono text-[10px] text-white/45">{{ activeSub.url }}</div>
      <SubUsage :sub="activeSub" />
      <div class="mt-2 font-mono text-[10px] text-white/45">Updated {{ updatedText }}</div>
    </div>

    <div class="mb-3 flex gap-2">
      <label class="flex flex-1 items-center gap-2 rounded-lg border border-ink-line bg-black/30 px-2 text-white/50 focus-within:border-accent/60">
        <Search :size="13" />
        <input v-model="query" placeholder="Search nodes" class="w-full bg-transparent py-1.5 text-xs text-white outline-none placeholder:text-white/30" />
      </label>
      <button class="btn" :class="{ '!border-accent/50 !text-accent-soft': sortByPing }" :aria-pressed="sortByPing" @click="sortByPing = !sortByPing">
        <ArrowDownUp :size="13" /> Ping
      </button>
    </div>

    <div class="-mr-2 flex-1 space-y-2 overflow-y-auto pr-2">
      <div v-if="!store.nodes.length" class="grid h-full place-items-center text-center text-xs text-white/40">
        No nodes yet.<br />Import a subscription or paste a config link.
      </div>
      <div v-else-if="!visible.length" class="grid h-full place-items-center text-center text-xs text-white/40">No nodes match.</div>
      <div
        v-for="n in visible"
        :key="n.id"
        class="group flex cursor-pointer items-center gap-3 rounded-xl border px-3 py-2 transition"
        :class="store.settings?.selected_node === n.id ? 'border-accent/50 bg-accent/5' : 'border-ink-line bg-white/[0.02] hover:border-white/20'"
        @click="selectNode(n.id)"
      >
        <span class="rounded border border-ink-line px-1.5 py-0.5 font-mono text-[9px] uppercase text-accent-soft">{{ n.protocol }}</span>
        <div class="min-w-0 flex-1">
          <div class="truncate text-sm text-white/90">{{ n.name }}</div>
          <div class="flex items-center gap-2 truncate font-mono text-[10px] text-white/45">
            <span class="truncate">{{ n.server }}:{{ n.port }}</span>
            <span v-if="n.up_total || n.down_total" class="flex shrink-0 items-center gap-1.5" title="Total traffic through this node">
              <span class="flex items-center"><ArrowDown :size="9" />{{ fmtBytes(n.down_total) }}</span>
              <span class="flex items-center"><ArrowUp :size="9" />{{ fmtBytes(n.up_total) }}</span>
            </span>
          </div>
        </div>
        <div class="w-16 shrink-0 text-right font-mono leading-tight">
          <div class="text-xs" :class="latencyClass(n.delay !== null && n.delay < 0 ? 99999 : n.delay, 400, 800)" title="Real HTTP delay">{{ delayText(n.delay) }}</div>
          <div class="text-[10px]" :class="latencyClass(n.latency)" title="TCP ping">tcp {{ n.latency === null ? "—" : n.latency }}</div>
        </div>
        <button
          v-if="n.link"
          class="text-white/30 opacity-0 transition hover:text-white focus-visible:opacity-100 group-hover:opacity-100"
          aria-label="Show QR code"
          @click.stop="store.ui.qr = { title: n.name, text: n.link }"
        >
          <QrCode :size="14" />
        </button>
        <button class="text-white/30 opacity-0 transition hover:text-red-400 focus-visible:opacity-100 group-hover:opacity-100" aria-label="Remove node" @click.stop="removeNode(n.id)">
          <Trash2 :size="14" />
        </button>
      </div>
    </div>
  </div>
</template>
