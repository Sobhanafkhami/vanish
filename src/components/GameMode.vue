<script setup lang="ts">
import { t } from "../lib/i18n";
import { computed } from "vue";
import { AppWindow, Gamepad2, Plus, X } from "lucide-vue-next";
import Toggle from "./Toggle.vue";
import { removeGameApp, setGameMode, store } from "../stores/app";

const on = computed(() => !!store.settings?.game_mode);
const g = computed(() => store.game);

const tone = (ms: number | null, good: number, ok: number) =>
  ms === null ? "text-white/40" : ms <= good ? "text-accent-soft" : ms <= ok ? "text-yellow-300" : "text-red-400";

// One verdict a player can read at a glance; ping, jitter and loss all matter for feel.
const verdict = computed(() => {
  const s = g.value;
  if (!store.status.running) return { text: "Connect to start monitoring.", tone: "text-white/45" };
  if (!s || s.ping === null) return { text: s ? "No answer yet" : "Measuring…", tone: "text-white/45" };
  if (s.loss >= 10 || s.ping > 200 || s.jitter > 60) return { text: "Laggy", tone: "text-red-400" };
  if (s.loss >= 3 || s.ping > 120 || s.jitter > 30) return { text: "Playable", tone: "text-yellow-300" };
  if (s.ping > 60 || s.jitter > 15) return { text: "Good", tone: "text-accent-soft" };
  return { text: "Excellent", tone: "text-accent-soft" };
});

const ceiling = computed(() => Math.max(100, ...(g.value?.samples ?? [0])));
const bars = computed(() =>
  (g.value?.samples ?? []).map((v) => (v < 0 ? { lost: true, h: 100 } : { lost: false, h: Math.max(6, Math.round((v / ceiling.value) * 100)) })),
);
</script>

<template>
  <div class="panel shrink-0 overflow-hidden px-4 py-3" :class="{ 'game-on': on }">
    <div class="flex items-center justify-between gap-3">
      <span class="flex items-center gap-2">
        <span class="relative flex h-7 w-7 shrink-0 items-center justify-center rounded-lg transition-colors duration-300" :class="on ? 'bg-accent/15 text-accent-soft' : 'bg-white/5 text-white/50'">
          <span v-if="on" class="absolute inset-0 animate-ping rounded-lg bg-accent/20" aria-hidden="true"></span>
          <Gamepad2 :size="16" class="relative block" />
        </span>
        <span>
          <span class="block text-sm font-semibold text-white">{{ t("Game mode") }}</span>
          <span class="block text-[11px] leading-tight" :class="on ? verdict.tone : 'text-white/45'">
            {{ on ? t(verdict.text) : t("Low lag and a steady connection for online games.") }}
          </span>
        </span>
      </span>
      <Toggle :model-value="on" :label="t('Game mode')" :disabled="store.busy" @update:model-value="setGameMode" />
    </div>

    <div class="grid transition-[grid-template-rows] duration-300 ease-out" :class="on ? 'grid-rows-[1fr]' : 'grid-rows-[0fr]'">
      <div class="min-h-0 overflow-hidden">
        <div class="pt-3">
          <div class="grid grid-cols-3 gap-2 text-center">
            <div class="rounded-lg bg-black/30 px-2 py-2">
              <div class="font-mono text-lg leading-none transition-colors duration-300" :class="tone(g?.ping ?? null, 60, 120)">{{ g?.ping ?? "—" }}</div>
              <div class="mt-1 text-[10px] text-white/45">{{ t("Ping") }} · {{ t("ms") }}</div>
            </div>
            <div class="rounded-lg bg-black/30 px-2 py-2">
              <div class="font-mono text-lg leading-none transition-colors duration-300" :class="g ? tone(g.jitter, 15, 30) : 'text-white/40'">{{ g ? g.jitter : "—" }}</div>
              <div class="mt-1 text-[10px] text-white/45">{{ t("Jitter") }} · {{ t("ms") }}</div>
            </div>
            <div class="rounded-lg bg-black/30 px-2 py-2">
              <div class="font-mono text-lg leading-none transition-colors duration-300" :class="g ? tone(g.loss, 0, 5) : 'text-white/40'">{{ g ? g.loss : "—" }}<span class="text-[10px]">%</span></div>
              <div class="mt-1 text-[10px] text-white/45">{{ t("Loss") }}</div>
            </div>
          </div>

          <div dir="ltr" class="mt-3 flex h-10 items-end gap-[2px]" aria-hidden="true">
            <span
              v-for="(b, i) in bars"
              :key="i"
              class="flex-1 rounded-sm transition-all duration-500"
              :class="b.lost ? 'bg-red-500/70' : 'bg-accent/50'"
              :style="{ height: b.h + '%' }"
            ></span>
            <span v-if="!bars.length" class="h-px w-full bg-white/10"></span>
          </div>

          <div class="mt-3">
            <div class="mb-1.5 flex items-center justify-between">
              <span class="hud-label">{{ t("Games") }}</span>
              <button class="flex items-center gap-1 text-[11px] text-accent-soft transition hover:text-white" @click="store.ui.appPicker = 'game'">
                <Plus :size="12" /> {{ t("Add game") }}
              </button>
            </div>
            <TransitionGroup name="chip" tag="div" class="flex flex-wrap gap-1.5">
              <span
                v-for="a in store.settings?.game_apps ?? []"
                :key="a"
                class="inline-flex items-center gap-1.5 rounded-lg border border-ink-line bg-black/40 py-0.5 ps-2 pe-1 font-mono text-[11px] text-white/85"
              >
                <AppWindow :size="11" class="text-accent-soft" /> {{ a }}
                <button class="rounded p-0.5 text-white/40 transition hover:text-red-400" :aria-label="t('Remove')" @click="removeGameApp(a)"><X :size="11" /></button>
              </span>
            </TransitionGroup>
            <p class="mt-1 text-[11px] leading-snug text-white/45">
              {{ store.settings?.game_apps.length ? t("Only these games use the tunnel. Everything else stays direct, so it never competes with them.") : t("No games added: all traffic uses the tunnel.") }}
            </p>
          </div>

          <p class="mt-2 border-t border-ink-line pt-2 text-[11px] leading-snug text-white/40">{{ t("Uses TUN, so run Vanish as administrator. If the server stops answering, Vanish switches to the next-best one within seconds.") }}</p>
        </div>
      </div>
    </div>
  </div>
</template>
