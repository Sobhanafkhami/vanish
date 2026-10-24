<script setup lang="ts">
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Minus, ScrollText, Settings, Square, X } from "lucide-vue-next";
import { store } from "../stores/app";

const win = getCurrentWindow();
</script>

<template>
  <header
    data-tauri-drag-region
    class="flex h-10 shrink-0 items-center justify-between border-b border-ink-line bg-black/40 pl-4"
  >
    <div data-tauri-drag-region class="pointer-events-none flex items-center gap-2">
      <span class="h-2 w-2 rounded-full bg-accent"></span>
      <span class="font-mono text-xs font-semibold uppercase tracking-[0.3em] text-white/80">Vanish</span>
    </div>
    <div class="flex h-full">
      <button class="w-11 text-white/50 transition hover:bg-white/10 hover:text-white" aria-label="Core log" @click="store.ui.logs = true">
        <ScrollText :size="14" class="mx-auto" />
      </button>
      <button class="w-11 text-white/50 transition hover:bg-white/10 hover:text-white" aria-label="Settings" @click="store.ui.settings = true">
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
