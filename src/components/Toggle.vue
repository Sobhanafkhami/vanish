<script setup lang="ts">
defineProps<{ modelValue: boolean; label: string; disabled?: boolean }>();
defineEmits<{ "update:modelValue": [value: boolean] }>();
</script>

<template>
  <button
    type="button"
    role="switch"
    :aria-checked="modelValue"
    :aria-label="label"
    :disabled="disabled"
    class="relative h-5 w-9 shrink-0 rounded-full border transition-colors duration-200 disabled:opacity-40"
    :class="modelValue ? 'border-accent/60 bg-accent/25' : 'border-ink-line bg-black/50 hover:border-white/25'"
    @click="$emit('update:modelValue', !modelValue)"
  >
    <span
      class="absolute left-0.5 top-0.5 h-3.5 w-3.5 rounded-full transition-all duration-200"
      :class="modelValue ? 'translate-x-4 bg-accent-soft shadow-[0_0_8px_rgba(74,222,128,.55)]' : 'translate-x-0 bg-white/40'"
    ></span>
  </button>
</template>
