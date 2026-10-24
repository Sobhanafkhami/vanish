<script setup lang="ts">
import Modal from "./Modal.vue";
import { store, updateSettings } from "../stores/app";
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
</script>

<template>
  <Modal v-if="store.settings" title="Settings" wide @close="store.ui.settings = false">
    <p v-if="store.status.running" class="mb-3 rounded-lg border border-accent/30 bg-accent/5 px-3 py-2 text-xs text-accent-soft">
      Routing, DNS and port changes apply the next time you connect.
    </p>

    <section class="mb-5 space-y-3">
      <h3 class="hud-label">General</h3>
      <label v-for="t in toggles" :key="t.key" class="flex cursor-pointer items-start justify-between gap-4">
        <span>
          <span class="block text-sm text-white/90">{{ t.label }}</span>
          <span class="block text-xs text-white/50">{{ t.hint }}</span>
        </span>
        <input
          type="checkbox"
          role="switch"
          class="mt-1 h-4 w-4 shrink-0 cursor-pointer accent-green-500"
          :checked="store.settings[t.key]"
          @change="updateSettings({ [t.key]: ($event.target as HTMLInputElement).checked })"
        />
      </label>
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
      <p class="text-xs text-white/50">{{ routes.find((r) => r.id === store.settings!.route_mode)?.hint }}</p>

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
      <label class="block">
        <span class="text-sm text-white/90">Direct apps</span>
        <span class="block text-xs text-white/50">Process names that bypass the proxy, one per line (e.g. steam.exe). Needs TUN or System Proxy to see the process.</span>
        <textarea
          rows="2"
          class="mt-1 w-full resize-none rounded-lg border border-ink-line bg-black/40 p-2 font-mono text-xs text-white outline-none focus:border-accent/60"
          :value="store.settings.direct_apps.join('\n')"
          @change="updateSettings({ direct_apps: lines(($event.target as HTMLTextAreaElement).value) })"
        />
      </label>
    </section>

    <section class="space-y-3">
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
  </Modal>
</template>
