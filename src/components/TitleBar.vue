<script setup lang="ts">
import { lang, setLang, t } from "../lib/i18n";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Minus, ScrollText, Settings, Square, X } from "lucide-vue-next";
import { store } from "../stores/app";
import logo from "../assets/logo.png";

const win = getCurrentWindow();
</script>

<template>
  <header
    dir="ltr"
    data-tauri-drag-region
    class="flex h-10 shrink-0 items-center justify-between border-b border-ink-line bg-black/40 ps-4"
  >
    <div data-tauri-drag-region class="pointer-events-none flex items-center gap-2">
      <img :src="logo" alt="" class="h-5 w-5 rounded" />
      <span class="font-mono text-xs font-semibold uppercase tracking-[0.3em] text-white/80">{{ t("Vanish") }}</span>
    </div>
    <div class="flex h-full">
      <button
        class="w-11 font-mono text-[11px] text-white/50 transition hover:bg-white/10 hover:text-white"
        :aria-label="t('Language')"
        :title="t('Language')"
        @click="setLang(lang === 'fa' ? 'en' : 'fa')"
      >
        {{ lang === "fa" ? "EN" : "فا" }}
      </button>
      <button class="w-11 text-white/50 transition hover:bg-white/10 hover:text-white" :aria-label="t('Core log')" @click="store.ui.logs = true">
        <ScrollText :size="14" class="mx-auto" />
      </button>
      <button class="w-11 text-white/50 transition hover:bg-white/10 hover:text-white" :aria-label="t('Settings')" @click="store.ui.settings = true">
        <Settings :size="14" class="mx-auto" />
      </button>
      <button class="w-11 text-white/50 transition hover:bg-white/10 hover:text-white" @click="win.minimize()">
        <Minus :size="14" class="mx-auto" />
      </button>
      <button class="w-11 text-white/50 transition hover:bg-white/10 hover:text-white" @click="win.toggleMaximize()">
        <Square :size="11" class="mx-auto" />
      </button>
      <!-- Close hides to the system tray (see on_window_event in lib.rs). -->
      <button class="w-11 text-white/50 transition hover:bg-red-500/70 hover:text-white" @click="win.hide()">
        <X :size="15" class="mx-auto" />
      </button>
    </div>
  </header>
</template>
