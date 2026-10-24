<script setup lang="ts">
import { onMounted, ref } from "vue";
import { X } from "lucide-vue-next";

defineProps<{ title: string; wide?: boolean }>();
defineEmits<{ close: [] }>();

const root = ref<HTMLElement | null>(null);
onMounted(() => root.value?.focus()); // so Esc closes the dialog
</script>

<template>
  <div ref="root" tabindex="-1" class="absolute inset-0 z-40 outline-none grid place-items-center bg-black/70 p-6" role="dialog" aria-modal="true" @click.self="$emit('close')" @keydown.esc="$emit('close')">
    <div class="panel flex max-h-full w-full flex-col overflow-hidden" :class="wide ? 'max-w-2xl' : 'max-w-sm'">
      <div class="flex items-center justify-between border-b border-ink-line px-4 py-3">
        <span class="text-sm font-medium text-white">{{ title }}</span>
        <button class="text-white/50 hover:text-white" aria-label="Close" @click="$emit('close')"><X :size="16" /></button>
      </div>
      <div class="min-h-0 flex-1 overflow-y-auto p-4"><slot /></div>
    </div>
  </div>
</template>
