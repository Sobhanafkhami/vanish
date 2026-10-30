<script setup lang="ts">
import { t } from "../lib/i18n";
import { computed, nextTick, ref } from "vue";
import { Activity, ArrowDown, ArrowDownUp, ArrowUp, Check, ChevronDown, ClipboardPaste, EllipsisVertical, FileText, Link2, Pencil, Plus, QrCode, RefreshCw, Search, Timer, Trash2, X, Zap } from "lucide-vue-next";
import type { VanishNode } from "../lib/api";
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
const showTest = ref(false);
const showImportMenu = ref(false);
// Phones have no hover, so a row's extra actions live in a bottom sheet opened by its ⋯ button.
const sheetNode = ref<VanishNode | null>(null);
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

/** One tap: take whatever is on the clipboard, a subscription link or a pile of configs, and import it. */
async function importFromClipboard() {
  showImportMenu.value = false;
  let text = "";
  try {
    text = (await navigator.clipboard.readText()).trim();
  } catch {
    flash(t("Clipboard access was denied"));
    return;
  }
  if (!text) {
    flash(t("The clipboard is empty"));
    return;
  }
  flash(/^https?:\/\/\S+$/i.test(text) ? await addSubscription("", text) : await importLinks(text));
}

function openImportForm(kind: "sub" | "links") {
  showImportMenu.value = false;
  importKind.value = kind;
  showImport.value = true;
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
    flash(t("Clipboard access was denied"));
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
  return d < 0 ? t("timeout") : `${d} ms`;
}

const updatedText = computed(() => {
  const at = activeSub.value?.updated_at;
  return at ? new Date(at * 1000).toLocaleString() : t("never");
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
  if (confirm(t('Remove "{name}" and its {n} node(s)?', { name: activeSub.value.name, n: countFor(activeSub.value.id) }))) await removeSubscription(activeSub.value.id);
}
</script>

<template>
  <div class="panel flex min-h-0 flex-1 flex-col p-4 max-md:!border-0 max-md:!bg-transparent max-md:!p-0">
    <div class="mb-3 flex flex-wrap items-center justify-between gap-x-2 gap-y-2">
      <h2 class="flex items-baseline gap-2 text-sm font-semibold text-white">
        {{ t("Servers") }}
        <span class="font-mono text-xs font-normal text-white/45">{{ visible.length }}</span>
      </h2>
      <div class="relative grid w-full grid-cols-3 items-stretch gap-2 md:flex md:w-auto md:items-center">
        <button class="btn justify-center !border-accent/40 !text-accent-soft" :disabled="store.busy || !store.nodes.length" :title="t('Ping all, then select the quickest')" @click="connectFastest">
          <Zap :size="13" /> {{ t("Fastest") }}
        </button>
        <div class="md:relative">
          <button class="btn w-full justify-center" :aria-expanded="showTest" :disabled="!store.nodes.length" @click="showTest = !showTest">
            <Timer :size="13" /> {{ store.delayTesting ? t("Testing…") : t("Test") }} <ChevronDown :size="12" class="transition" :class="{ 'rotate-180': showTest }" />
          </button>
          <template v-if="showTest">
            <button class="fixed inset-0 z-10 cursor-default" tabindex="-1" aria-hidden="true" @click="showTest = false"></button>
            <div class="absolute top-full z-20 mt-1 rounded-xl border border-ink-line bg-ink-raised p-1 shadow-xl max-md:inset-x-0 md:end-0 md:w-64" role="menu">
              <button class="menu-item" role="menuitem" :disabled="store.busy" @click="showTest = false; pingAll()">
                <Activity :size="14" class="mt-0.5 shrink-0 text-accent-soft" />
                <span><span class="block text-xs text-white/90">{{ t("TCP ping") }}</span><span class="block text-[11px] text-white/45">{{ t("TCP connect time to each server") }}</span></span>
              </button>
              <button class="menu-item" role="menuitem" :disabled="store.delayTesting || !visible.length" @click="showTest = false; realDelay(visible.map((n) => n.id))">
                <Timer :size="14" class="mt-0.5 shrink-0 text-accent-soft" />
                <span><span class="block text-xs text-white/90">{{ t("Real delay") }}</span><span class="block text-[11px] text-white/45">{{ t("Real HTTP request through each node, like URL test") }}</span></span>
              </button>
            </div>
          </template>
        </div>
        <div class="md:relative">
          <button class="btn w-full justify-center" :class="{ '!border-accent/50 !text-accent-soft': showImport || showImportMenu }" :aria-expanded="showImportMenu" @click="showImportMenu = !showImportMenu">
            <Plus :size="13" /> {{ t("Import") }}
          </button>
          <template v-if="showImportMenu">
            <button class="fixed inset-0 z-10 cursor-default" tabindex="-1" aria-hidden="true" @click="showImportMenu = false"></button>
            <div class="absolute top-full z-20 mt-1 rounded-xl border border-ink-line bg-ink-raised p-1 shadow-xl max-md:inset-x-0 md:end-0 md:w-64" role="menu">
              <button class="menu-item" role="menuitem" @click="importFromClipboard">
                <ClipboardPaste :size="16" class="mt-0.5 shrink-0 text-accent-soft" />
                <span><span class="block text-sm text-white/90 md:text-xs">{{ t("Import from clipboard") }}</span><span class="block text-[11px] text-white/45">{{ t("A subscription link or copied configs") }}</span></span>
              </button>
              <button class="menu-item" role="menuitem" @click="openImportForm('sub')">
                <Link2 :size="16" class="mt-0.5 shrink-0 text-accent-soft" />
                <span><span class="block text-sm text-white/90 md:text-xs">{{ t("Add subscription link") }}</span><span class="block text-[11px] text-white/45">{{ t("Type or paste a subscription URL") }}</span></span>
              </button>
              <button class="menu-item" role="menuitem" @click="openImportForm('links')">
                <FileText :size="16" class="mt-0.5 shrink-0 text-accent-soft" />
                <span><span class="block text-sm text-white/90 md:text-xs">{{ t("Paste configs") }}</span><span class="block text-[11px] text-white/45">{{ t("vless, vmess, trojan, ss links or JSON") }}</span></span>
              </button>
            </div>
          </template>
        </div>
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
              {{ t(k.label) }}
            </button>
          </div>
          <button class="btn" @click="paste"><ClipboardPaste :size="13" /> {{ t("Paste") }}</button>
        </div>

        <div v-if="importKind === 'sub'" class="space-y-2">
          <input
            v-model="subName"
            maxlength="40"
            :placeholder="t('Name (e.g. Work, Home)')"
            class="w-full rounded-lg border border-ink-line bg-black/40 p-2 text-xs text-white outline-none placeholder:text-white/30 focus:border-accent/60"
          />
          <input dir="ltr"
            v-model="subUrl"
            :placeholder="t('https://example.com/sub/...')"
            class="w-full rounded-lg border border-ink-line bg-black/40 p-2 font-mono text-xs text-white outline-none placeholder:text-white/30 focus:border-accent/60"
            @keydown.enter="doImport"
          />
        </div>
        <textarea dir="ltr"
          v-else
          v-model="input"
          rows="3"
          :placeholder="t('vless:// vmess:// trojan:// ss:// wireguard:// links, or sing-box outbound JSON')"
          class="w-full resize-none rounded-lg border border-ink-line bg-black/40 p-2 font-mono text-xs text-white outline-none placeholder:text-white/30 focus:border-accent/60"
        />
        <button class="btn mt-2" :disabled="store.busy" @click="doImport">{{ importKind === "sub" ? t("Add subscription") : t("Add") }}</button>
      </div>
    </Transition>
    <div v-if="message" class="mb-2 font-mono text-xs text-accent-soft">{{ message }}</div>

    <!-- tabs: one per subscription -->
    <div v-if="store.subs.length || manualCount" class="mb-3 flex gap-1 overflow-x-auto border-b border-ink-line pb-2" role="tablist">
      <button
        role="tab"
        :aria-selected="store.activeTab === 'all'"
        class="shrink-0 rounded-md px-3 py-2 text-xs transition md:py-1"
        :class="store.activeTab === 'all' ? 'bg-accent/15 text-accent-soft' : 'text-white/50 hover:text-white/80'"
        @click="store.activeTab = 'all'"
      >
        {{ t("All") }} <span class="ms-1 font-mono text-[10px] opacity-70">{{ store.nodes.length }}</span>
      </button>
      <button
        v-if="manualCount"
        role="tab"
        :aria-selected="store.activeTab === 'manual'"
        class="shrink-0 rounded-md px-3 py-2 text-xs transition md:py-1"
        :class="store.activeTab === 'manual' ? 'bg-accent/15 text-accent-soft' : 'text-white/50 hover:text-white/80'"
        @click="store.activeTab = 'manual'"
      >
        {{ t("Manual") }} <span class="ms-1 font-mono text-[10px] opacity-70">{{ manualCount }}</span>
      </button>
      <button
        v-for="s in store.subs"
        :key="s.id"
        role="tab"
        :aria-selected="store.activeTab === s.id"
        class="max-w-[10rem] shrink-0 truncate rounded-md px-3 py-2 text-xs transition md:py-1"
        :class="store.activeTab === s.id ? 'bg-accent/15 text-accent-soft' : 'text-white/50 hover:text-white/80'"
        @click="store.activeTab = s.id"
      >
        {{ s.name }} <span class="ms-1 font-mono text-[10px] opacity-70">{{ countFor(s.id) }}</span>
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
            <button class="p-2 hover:text-accent-soft md:p-1" :aria-label="t('Save name')" @click="commitRename"><Check :size="14" /></button>
            <button class="p-2 hover:text-white md:p-1" :aria-label="t('Cancel')" @click="renaming = false"><X :size="14" /></button>
          </template>
          <template v-else>
            <button class="p-2 hover:text-white md:p-1" :aria-label="t('Rename subscription')" @click="startRename"><Pencil :size="14" /></button>
            <button class="p-2 hover:text-white md:p-1" :aria-label="t('Show subscription QR')" @click="store.ui.qr = { title: activeSub.name, text: activeSub.url }">
              <QrCode :size="14" />
            </button>
            <button class="p-2 hover:text-accent-soft disabled:opacity-40 md:p-1" :disabled="store.busy" :aria-label="t('Refresh subscription')" @click="refreshSubscription(activeSub.id)">
              <RefreshCw :size="14" />
            </button>
            <button class="p-2 hover:text-red-400 md:p-1" :aria-label="t('Remove subscription')" @click="deleteSub"><Trash2 :size="14" /></button>
          </template>
        </div>
      </div>
      <div class="mt-1 truncate font-mono text-[10px] text-white/45">{{ activeSub.url }}</div>
      <SubUsage :sub="activeSub" />
      <div class="mt-2 font-mono text-[10px] text-white/45">{{ t("Updated") }} {{ updatedText }}</div>
    </div>

    <div class="mb-3 flex gap-2">
      <label class="flex flex-1 items-center gap-2 rounded-lg border border-ink-line bg-black/30 px-2 text-white/50 focus-within:border-accent/60">
        <Search :size="13" />
        <input v-model="query" :placeholder="t('Search nodes')" class="w-full bg-transparent py-2.5 text-xs text-white outline-none placeholder:text-white/30 md:py-1.5" />
      </label>
      <button class="btn" :class="{ '!border-accent/50 !text-accent-soft': sortByPing }" :aria-pressed="sortByPing" @click="sortByPing = !sortByPing">
        <ArrowDownUp :size="13" /> {{ t("Ping") }}
      </button>
    </div>

    <div class="-me-2 flex-1 space-y-2 overflow-y-auto pe-2">
      <div v-if="!store.nodes.length" class="grid h-full place-items-center text-center">
        <div>
          <p class="text-sm text-white/70">{{ t("No nodes yet.") }}</p>
          <p class="mt-1 text-xs text-white/40">{{ t("Import a subscription or paste a config link.") }}</p>
          <button class="btn mt-3" @click="showImport = true"><Plus :size="13" /> {{ t("Import") }}</button>
        </div>
      </div>
      <div v-else-if="!visible.length" class="grid h-full place-items-center text-center text-xs text-white/40">{{ t("No nodes match.") }}</div>
      <TransitionGroup name="row" tag="div" class="relative space-y-2">
      <div
        v-for="(n, i) in visible"
        :key="n.id"
        :style="{ '--i': Math.min(i, 12) }"
        class="group relative flex cursor-pointer items-center gap-3 rounded-xl border px-3 py-3 transition md:py-2"
        :class="store.settings?.selected_node === n.id ? 'border-accent/50 bg-accent/10' : 'border-transparent bg-white/[0.03] hover:bg-white/[0.06]'"
        @click="selectNode(n.id)"
      >
        <span v-if="store.settings?.selected_node === n.id" class="absolute inset-y-3 start-0 w-[3px] rounded-full bg-accent md:hidden" aria-hidden="true"></span>
        <span class="hidden w-14 shrink-0 rounded bg-white/5 px-1.5 py-0.5 text-center font-mono text-[10px] uppercase text-white/60 md:block">{{ n.protocol }}</span>
        <div class="min-w-0 flex-1">
          <!-- the whole name on phones (wraps), one truncated line on desktop -->
          <div class="break-words text-[15px] leading-snug md:truncate md:text-sm" :class="store.settings?.selected_node === n.id ? 'font-medium text-white' : 'text-white/90'">{{ n.name }}</div>
          <div class="mt-1 flex flex-wrap items-center gap-x-2 gap-y-0.5 font-mono text-[10px] text-white/45 md:flex-nowrap">
            <span class="rounded bg-white/5 px-1.5 py-px uppercase text-white/60 md:hidden">{{ n.protocol }}</span>
            <span class="break-all md:truncate">{{ n.server }}:{{ n.port }}</span>
            <span v-if="n.up_total || n.down_total" class="flex shrink-0 items-center gap-1.5" :title="t('Total traffic through this node')">
              <span class="flex items-center"><ArrowDown :size="9" />{{ fmtBytes(n.down_total) }}</span>
              <span class="flex items-center"><ArrowUp :size="9" />{{ fmtBytes(n.up_total) }}</span>
            </span>
          </div>
        </div>
        <div class="min-w-[4.5rem] shrink-0 whitespace-nowrap text-end font-mono leading-tight md:w-24" :title="n.delay !== null && n.delay > 0 ? t('Real HTTP delay through this server') : t('TCP connect time')">
          <template v-if="n.delay !== null">
            <div class="text-sm" :class="latencyClass(n.delay < 0 ? 99999 : n.delay, 400, 800)">{{ delayText(n.delay) }}</div>
          </template>
          <template v-else-if="n.latency !== null">
            <div class="text-sm" :class="latencyClass(n.latency)">{{ n.latency }} <span class="text-[10px] text-white/40">{{ t("ms") }}</span></div>
            <div class="text-[10px] text-white/35">{{ t("tcp") }}</div>
          </template>
          <div v-else class="text-sm text-white/25">{{ store.delayTesting ? "…" : "—" }}</div>
        </div>
        <button class="-me-1 grid h-10 w-9 shrink-0 place-items-center rounded-lg text-white/50 active:bg-white/10 md:hidden" :aria-label="t('More')" @click.stop="sheetNode = n">
          <EllipsisVertical :size="18" />
        </button>
        <button
          v-if="n.link"
          class="hidden text-white/30 opacity-0 transition hover:text-white focus-visible:opacity-100 group-hover:opacity-100 md:block"
          :aria-label="t('Show QR code')"
          @click.stop="store.ui.qr = { title: n.name, text: n.link }"
        >
          <QrCode :size="14" />
        </button>
        <button class="hidden text-white/30 opacity-0 transition hover:text-red-400 focus-visible:opacity-100 group-hover:opacity-100 md:block" :aria-label="t('Remove node')" @click.stop="removeNode(n.id)">
          <Trash2 :size="14" />
        </button>
      </div>
      </TransitionGroup>
    </div>

    <Teleport to="body">
      <Transition name="fade">
        <div v-if="sheetNode" class="fixed inset-0 z-50 flex items-end bg-black/60 md:hidden" @click.self="sheetNode = null">
          <div class="w-full rounded-t-2xl border-t border-ink-line bg-ink-raised p-3" :style="{ paddingBottom: 'calc(0.75rem + env(safe-area-inset-bottom))' }" role="dialog">
            <div class="truncate px-3 pb-2 text-sm font-medium text-white">{{ sheetNode.name }}</div>
            <button
              v-if="sheetNode.link"
              class="menu-item items-center !py-3"
              @click="store.ui.qr = { title: sheetNode.name, text: sheetNode.link }; sheetNode = null"
            >
              <QrCode :size="18" class="text-accent-soft" /> <span class="text-sm text-white/90">{{ t("Show QR code") }}</span>
            </button>
            <button class="menu-item items-center !py-3" @click="removeNode(sheetNode.id); sheetNode = null">
              <Trash2 :size="18" class="text-red-400" /> <span class="text-sm text-red-300">{{ t("Remove node") }}</span>
            </button>
            <button class="menu-item items-center justify-center !py-3" @click="sheetNode = null"><span class="text-sm text-white/60">{{ t("Cancel") }}</span></button>
          </div>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>
