<script setup lang="ts">
import { onBeforeUnmount, ref } from "vue";
import { AppWindow, Plus, RotateCcw, X } from "lucide-vue-next";
import Modal from "./Modal.vue";
import Toggle from "./Toggle.vue";
import { addDirectApp, checkForUpdate, installUpdate, resetSettings, store, updateSettings } from "../stores/app";
import type { Settings } from "../lib/api";

type BoolKey = "autostart" | "connect_on_launch" | "auto_fallback" | "kill_switch";

const toggles: { key: BoolKey; label: string; hint: string }[] = [
  { key: "autostart", label: "Start with Windows", hint: "Launches minimized to the tray at login." },
  { key: "connect_on_launch", label: "Connect on launch", hint: "Connects to the selected node as soon as the app starts." },
  { key: "auto_fallback", label: "Auto fallback", hint: "Switches to the next-fastest node when the current one stops responding." },
  { key: "kill_switch", label: "Kill switch", hint: "System Proxy mode: if the core dies, traffic is blocked instead of leaking." },
];

const routes: { id: Settings["route_mode"]; label: string; hint: string }[] = [
  { id: "global", label: "Global", hint: "Everything through the proxy." },
  { id: "rule", label: "Rule", hint: ".ir sites and your lists below go direct." },
  { id: "direct", label: "Direct", hint: "Nothing is proxied." },
];

const refreshChoices = [
  { v: 0, label: "Off" },
  { v: 1, label: "1 hour" },
  { v: 6, label: "6 hours" },
  { v: 12, label: "12 hours" },
  { v: 24, label: "24 hours" },
];

const lines = (s: string) => s.split(/[\n,]/).map((x) => x.trim()).filter(Boolean);

function onPort(key: "mixed_port" | "api_port", e: Event) {
  const v = Number((e.target as HTMLInputElement).value);
  if (Number.isInteger(v) && v >= 1024 && v <= 65535) updateSettings({ [key]: v });
}

const appName = ref("");
async function addTyped() {
  await addDirectApp(appName.value);
  appName.value = "";
}
function removeApp(name: string) {
  updateSettings({ direct_apps: store.settings!.direct_apps.filter((a) => a !== name) });
}

// Reset needs a second click, and the armed state expires so a stray click never wipes settings.
const confirmReset = ref(false);
let resetTimer: ReturnType<typeof setTimeout> | undefined;
function onReset() {
  if (!confirmReset.value) {
    confirmReset.value = true;
    resetTimer = setTimeout(() => (confirmReset.value = false), 4000);
    return;
  }
  clearTimeout(resetTimer);
  confirmReset.value = false;
  resetSettings();
}
onBeforeUnmount(() => clearTimeout(resetTimer));

const updateStatus = ref("");
const checking = ref(false);
async function onCheck() {
  checking.value = true;
  updateStatus.value = await checkForUpdate();
  checking.value = false;
}
</script>

<template>
  <Modal v-if="store.settings" title="Settings" wide @close="store.ui.settings = false">
    <section class="mb-5 space-y-3">
      <h3 class="hud-label">General</h3>
      <div v-for="t in toggles" :key="t.key" class="flex items-start justify-between gap-4">
        <span>
          <span class="block text-sm text-white/90">{{ t.label }}</span>
          <span class="block text-xs text-white/50">{{ t.hint }}</span>
        </span>
        <Toggle class="mt-0.5" :label="t.label" :model-value="store.settings[t.key]" @update:model-value="updateSettings({ [t.key]: $event })" />
      </div>
      <label class="flex items-center justify-between gap-4">
        <span>
          <span class="block text-sm text-white/90">Refresh subscriptions</span>
          <span class="block text-xs text-white/50">How often saved subscriptions update in the background.</span>
        </span>
        <select
          class="cursor-pointer rounded-lg border border-ink-line bg-ink-raised px-2 py-1 text-xs text-white outline-none focus:border-accent/60"
          :value="store.settings.sub_refresh_hours"
          @change="updateSettings({ sub_refresh_hours: Number(($event.target as HTMLSelectElement).value) })"
        >
          <option v-for="c in refreshChoices" :key="c.v" :value="c.v">{{ c.label }}</option>
        </select>
      </label>
    </section>

    <section class="mb-5 space-y-3">
      <h3 class="hud-label">Routing</h3>
      <div class="flex gap-1 rounded-lg border border-ink-line bg-black/40 p-1">
        <button
          v-for="r in routes"
          :key="r.id"
          :aria-pressed="store.settings.route_mode === r.id"
          class="flex-1 rounded-md px-3 py-1.5 text-xs transition"
          :class="store.settings.route_mode === r.id ? 'bg-accent/15 text-accent-soft' : 'text-white/50 hover:text-white/80'"
          @click="updateSettings({ route_mode: r.id })"
        >
          {{ r.label }}
        </button>
      </div>
      <p class="text-xs text-white/50">{{ routes.find((r) => r.id === store.settings!.route_mode)?.hint }} Changes apply immediately, reconnecting if needed.</p>

      <label class="block">
        <span class="text-sm text-white/90">Direct domains</span>
        <span class="block text-xs text-white/50">One per line. Matches the domain and its subdomains (e.g. example.com).</span>
        <textarea
          rows="3"
          class="mt-1 w-full resize-none rounded-lg border border-ink-line bg-black/40 p-2 font-mono text-xs text-white outline-none focus:border-accent/60"
          :value="store.settings.direct_domains.join('\n')"
          @change="updateSettings({ direct_domains: lines(($event.target as HTMLTextAreaElement).value) })"
        />
      </label>

      <div>
        <span class="text-sm text-white/90">Direct apps</span>
        <span class="block text-xs text-white/50">These programs bypass the proxy.</span>
        <div class="mt-2 flex flex-wrap gap-2">
          <span
            v-for="a in store.settings.direct_apps"
            :key="a"
            class="inline-flex items-center gap-1.5 rounded-lg border border-ink-line bg-black/40 py-1 pl-2.5 pr-1.5 font-mono text-xs text-white/85"
          >
            <AppWindow :size="12" class="text-accent-soft" /> {{ a }}
            <button class="rounded p-0.5 text-white/40 transition hover:text-red-400" :aria-label="`Remove ${a}`" @click="removeApp(a)"><X :size="12" /></button>
          </span>
          <span v-if="!store.settings.direct_apps.length" class="text-xs text-white/40">No apps added.</span>
        </div>
        <div class="mt-2 flex gap-2">
          <input
            v-model="appName"
            placeholder="steam.exe"
            class="min-w-0 flex-1 rounded-lg border border-ink-line bg-black/40 px-2 py-1.5 font-mono text-xs text-white outline-none placeholder:text-white/30 focus:border-accent/60"
            @keydown.enter="addTyped"
          />
          <button class="btn" :disabled="!appName.trim()" @click="addTyped"><Plus :size="13" /> Add</button>
          <button class="btn" @click="store.ui.appPicker = true"><AppWindow :size="13" /> Choose app…</button>
        </div>
      </div>
    </section>

    <section class="mb-5 space-y-3">
      <h3 class="hud-label">Network</h3>
      <label class="flex items-center justify-between gap-4">
        <span class="text-sm text-white/90">DNS over HTTPS server</span>
        <input
          class="w-44 rounded-lg border border-ink-line bg-black/40 px-2 py-1 font-mono text-xs text-white outline-none focus:border-accent/60"
          :value="store.settings.dns_server"
          placeholder="1.1.1.1"
          @change="updateSettings({ dns_server: ($event.target as HTMLInputElement).value.trim() })"
        />
      </label>
      <label class="block">
        <span class="flex items-center justify-between gap-4">
          <span class="text-sm text-white/90">Test URL</span>
          <input
            class="w-72 rounded-lg border border-ink-line bg-black/40 px-2 py-1 font-mono text-xs text-white outline-none focus:border-accent/60"
            :value="store.settings.ping_url"
            @change="updateSettings({ ping_url: ($event.target as HTMLInputElement).value.trim() })"
          />
        </span>
        <span class="mt-1 block text-xs text-white/50">Used by Real delay and the latency check. Any HTTP(S) URL that returns quickly.</span>
      </label>
      <div class="grid grid-cols-2 gap-4">
        <label class="flex items-center justify-between gap-2">
          <span class="text-sm text-white/90">Local port</span>
          <input type="number" min="1024" max="65535" class="w-24 rounded-lg border border-ink-line bg-black/40 px-2 py-1 font-mono text-xs text-white outline-none focus:border-accent/60" :value="store.settings.mixed_port" @change="onPort('mixed_port', $event)" />
        </label>
        <label class="flex items-center justify-between gap-2">
          <span class="text-sm text-white/90">API port</span>
          <input type="number" min="1024" max="65535" class="w-24 rounded-lg border border-ink-line bg-black/40 px-2 py-1 font-mono text-xs text-white outline-none focus:border-accent/60" :value="store.settings.api_port" @change="onPort('api_port', $event)" />
        </label>
      </div>
    </section>

    <section class="mb-5 space-y-3">
      <h3 class="hud-label">About</h3>
      <div class="flex items-center justify-between gap-4">
        <span class="text-sm text-white/90">Vanish <span class="font-mono text-xs text-white/50">{{ store.version ? "v" + store.version : "" }}</span></span>
        <div class="flex items-center gap-2">
          <button v-if="store.update" class="btn !border-accent/50 !text-accent-soft" :disabled="store.updating" @click="installUpdate">
            {{ store.updating ? "Updating…" : `Install ${store.update.version}` }}
          </button>
          <button class="btn" :disabled="checking" @click="onCheck">{{ checking ? "Checking…" : "Check for updates" }}</button>
        </div>
      </div>
      <p v-if="updateStatus" class="text-xs text-white/55" role="status">{{ updateStatus }}</p>
    </section>

    <section class="border-t border-ink-line pt-4">
      <div class="flex items-center justify-between gap-4">
        <span>
          <span class="block text-sm text-white/90">Reset to default</span>
          <span class="block text-xs text-white/50">Restores every setting above. Your nodes and subscriptions are kept.</span>
        </span>
        <button class="btn shrink-0" :class="confirmReset ? '!border-red-500/60 !text-red-300' : ''" @click="onReset">
          <RotateCcw :size="13" /> {{ confirmReset ? "Click again to confirm" : "Reset" }}
        </button>
      </div>
    </section>
  </Modal>
</template>
