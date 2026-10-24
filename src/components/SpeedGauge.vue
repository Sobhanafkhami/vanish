<script setup lang="ts">
import { computed } from "vue";

const props = defineProps<{ value: number; color: string; unit?: string; caption?: string }>();

// Speedtest.net style: a 270° dial on a non-linear scale so both 5 and 500 Mbit/s are readable.
const MAX = 1000;
const TICKS = [0, 1, 5, 10, 25, 50, 100, 250, 500, 1000];
const CX = 120;
const CY = 118;
const R = 88;
const SWEEP = 270;
const START = 135;

const frac = (v: number) => Math.max(0, Math.min(1, Math.log10(1 + v) / Math.log10(1 + MAX)));
const filled = computed(() => frac(props.value) * 0.75 * 100);

const ticks = TICKS.map((t) => {
  const a = ((START + frac(t) * SWEEP) * Math.PI) / 180;
  return { t, x: CX + (R + 16) * Math.cos(a), y: CY + (R + 16) * Math.sin(a) + 3 };
});

const shown = computed(() => (props.value >= 100 ? props.value.toFixed(0) : props.value.toFixed(props.value >= 10 ? 1 : 2)));
</script>

<template>
  <svg viewBox="0 0 240 210" class="mx-auto block w-full max-w-[300px]" role="img" :aria-label="`${shown} ${unit ?? 'Mbit/s'}`">
    <g :transform="`rotate(${START} ${CX} ${CY})`" fill="none" stroke-linecap="round">
      <circle :cx="CX" :cy="CY" :r="R" pathLength="100" stroke="#1b2420" stroke-width="10" stroke-dasharray="75 100" />
      <circle
        :cx="CX"
        :cy="CY"
        :r="R"
        pathLength="100"
        :stroke="color"
        stroke-width="10"
        :stroke-dasharray="`${filled} 100`"
        style="transition: stroke-dasharray 0.25s ease-out; filter: drop-shadow(0 0 6px currentColor)"
        :style="{ color }"
      />
    </g>
    <text v-for="k in ticks" :key="k.t" :x="k.x" :y="k.y" text-anchor="middle" font-size="9" fill="rgba(255,255,255,.4)" font-family="ui-monospace, monospace">{{ k.t }}</text>
    <text :x="CX" :y="CY + 6" text-anchor="middle" font-size="40" font-weight="600" fill="#fff" font-family="ui-monospace, monospace">{{ shown }}</text>
    <text :x="CX" :y="CY + 26" text-anchor="middle" font-size="11" fill="rgba(255,255,255,.5)">{{ unit ?? "Mbit/s" }}</text>
    <text v-if="caption" :x="CX" :y="CY + 46" text-anchor="middle" font-size="10" letter-spacing="2" fill="rgba(255,255,255,.55)" font-family="ui-monospace, monospace">{{ caption }}</text>
  </svg>
</template>
