<script setup lang="ts">
import { t } from "../lib/i18n";
import { onMounted, ref } from "vue";
import { X } from "lucide-vue-next";

defineProps<{ title: string; wide?: boolean }>();
defineEmits<{ close: [] }>();

const root = ref<HTMLElement | null>(null);
onMounted(() => root.value?.focus()); // so Esc closes the dialog
</script>

<template>
  <div ref="root" tabindex="-1" class="absolute inset-0 z-40 grid place-items-center bg-black/70 p-0 outline-none md:p-6" role="dialog" aria-modal="true" @click.self="$emit('close')" @keydown.esc="$emit('close')">
    <div class="panel flex max-h-full w-full flex-col overflow-hidden" :class="[wide ? 'md:max-w-2xl' : 'md:max-w-sm', 'max-md:h-full max-md:max-w-none max-md:!rounded-none max-md:!border-0']">
      <div class="flex items-center justify-between border-b border-ink-line px-4 py-3" :style="{ paddingTop: 'calc(0.75rem + env(safe-area-inset-top))' }">
        <span class="text-sm font-medium text-white">{{ title }}</span>
        <button class="-me-2 grid h-10 w-10 place-items-center text-white/50 hover:text-white" :aria-label="t('Close')" @click="$emit('close')"><X :size="18" /></button>
      </div>
      <div class="min-h-0 flex-1 overflow-y-auto p-4" :style="{ paddingBottom: 'calc(1rem + env(safe-area-inset-bottom))' }"><slot /></div>
    </div>
  </div>
</template>
