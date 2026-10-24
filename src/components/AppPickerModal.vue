<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { Check, FolderOpen, Search } from "lucide-vue-next";
import Modal from "./Modal.vue";
import { api, type RunningApp } from "../lib/api";
import { addDirectApp, store } from "../stores/app";

const apps = ref<RunningApp[]>([]);
const loading = ref(true);
const error = ref("");
const query = ref("");

onMounted(async () => {
  try {
    apps.value = await api.listRunningApps();
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
});

const shown = computed(() => {
  const q = query.value.trim().toLowerCase();
  return q ? apps.value.filter((a) => `${a.name} ${a.title}`.toLowerCase().includes(q)) : apps.value;
});
const added = (name: string) => store.settings?.direct_apps.some((a) => a.toLowerCase() === name.toLowerCase()) ?? false;

async function browse() {
  try {
    const name = await api.pickAppExe();
    if (name) await addDirectApp(name);
  } catch (e) {
    error.value = String(e);
  }
}
</script>

<template>
  <Modal title="Choose an application" wide @close="store.ui.appPicker = false">
    <div class="mb-3 flex gap-2">
      <label class="flex flex-1 items-center gap-2 rounded-lg border border-ink-line bg-black/30 px-2 text-white/50 focus-within:border-accent/60">
        <Search :size="13" />
        <input v-model="query" placeholder="Search running apps" class="w-full bg-transparent py-1.5 text-xs text-white outline-none placeholder:text-white/30" />
      </label>
      <button class="btn" @click="browse"><FolderOpen :size="13" /> Browse…</button>
    </div>
    <p class="mb-2 text-xs text-white/50">Apps open on this PC. Traffic from the apps you pick bypasses the proxy.</p>

    <div class="max-h-72 space-y-1 overflow-y-auto">
      <div v-if="loading" class="py-6 text-center text-xs text-white/50">Loading…</div>
      <div v-else-if="error" class="py-6 text-center text-xs text-red-300">{{ error }}</div>
      <div v-else-if="!shown.length" class="py-6 text-center text-xs text-white/50">No matching apps. Use Browse to pick an .exe.</div>
      <button
        v-for="a in shown"
        :key="a.name"
        class="flex w-full items-center justify-between gap-3 rounded-lg border border-ink-line bg-white/[0.02] px-3 py-2 text-left transition hover:border-accent/40 disabled:opacity-60"
        :disabled="added(a.name)"
        @click="addDirectApp(a.name)"
      >
        <span class="min-w-0">
          <span class="block truncate font-mono text-xs text-white/90">{{ a.name }}</span>
          <span class="block truncate text-[11px] text-white/45">{{ a.title }}</span>
        </span>
        <span v-if="added(a.name)" class="flex shrink-0 items-center gap-1 text-xs text-accent-soft"><Check :size="13" /> Added</span>
        <span v-else class="shrink-0 text-xs text-white/50">Add</span>
      </button>
    </div>
  </Modal>
</template>
