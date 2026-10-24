<script setup lang="ts">
import { computed } from "vue";
import { ArrowDown, ArrowUp, CalendarClock } from "lucide-vue-next";
import type { Subscription } from "../lib/api";
import { fmtBytes, subUsage, usageTone } from "../lib/usage";
import { store } from "../stores/app";

const props = defineProps<{ sub: Subscription }>();

const u = computed(() => subUsage(props.sub));

// Traffic that went through this subscription's nodes in Vanish (always known, even when the panel reports nothing).
const local = computed(() => {
  const nodes = store.nodes.filter((n) => n.subscription === props.sub.id);
  return { down: nodes.reduce((a, n) => a + n.down_total, 0), up: nodes.reduce((a, n) => a + n.up_total, 0) };
});

const expiryText = computed(() => {
  if (u.value.expired) return "Expired";
  if (u.value.daysLeft === null) return null;
  return u.value.daysLeft === 0 ? "Expires today" : `${u.value.daysLeft} day${u.value.daysLeft === 1 ? "" : "s"} left`;
});
const expiryTone = computed(() =>
  u.value.expired || (u.value.daysLeft ?? 99) <= 3 ? "text-red-400" : (u.value.daysLeft ?? 99) <= 7 ? "text-yellow-300" : "text-white/60",
);
const label = computed(() => `Remaining ${u.value.remainingPct === null ? "" : Math.round(u.value.remainingPct) + " percent"}`);
</script>

<template>
  <div class="mt-3">
    <template v-if="u.kind !== 'none'">
      <div
        class="h-2 overflow-hidden rounded-full bg-white/10"
        role="progressbar"
        :aria-label="label"
        aria-valuemin="0"
        aria-valuemax="100"
        :aria-valuenow="u.remainingPct === null ? undefined : Math.round(u.remainingPct)"
      >
        <!-- green = what is left; the dark part is what has been used -->
        <div class="h-full rounded-full transition-[width] duration-500" :class="usageTone(u.remainingPct)" :style="{ width: (u.remainingPct ?? 0) + '%' }"></div>
      </div>

      <div class="mt-1.5 flex items-start justify-between gap-3 font-mono text-[11px]">
        <div class="text-white/60">
          <template v-if="u.used !== null">Used <span class="text-white/90">{{ u.kind === "remaining" ? "≈ " : "" }}{{ fmtBytes(u.used) }}</span></template>
          <template v-else-if="u.kind === 'remaining'">Used <span class="text-white/45">not reported</span></template>
        </div>
        <div class="text-right text-white/60">
          <template v-if="u.kind === 'unlimited'"><span class="text-accent-soft">Unlimited</span></template>
          <template v-else-if="u.remaining !== null">Remaining <span class="text-accent-soft">{{ fmtBytes(u.remaining) }}</span></template>
        </div>
      </div>

      <div class="mt-1 flex flex-wrap items-center justify-between gap-x-3 text-[10px] text-white/45">
        <span v-if="u.total">Total {{ fmtBytes(u.total) }}<template v-if="u.remainingPct !== null"> · {{ Math.round(100 - u.remainingPct) }}% used</template></span>
        <span v-else-if="u.kind === 'remaining'">Total not reported by this panel</span>
        <span v-if="expiryText" class="flex items-center gap-1" :class="expiryTone"><CalendarClock :size="11" /> {{ expiryText }}</span>
      </div>
    </template>
    <p v-else class="text-[11px] text-white/45">This subscription does not report traffic or expiry. Refresh it to check again.</p>

    <div v-if="local.down || local.up" class="mt-1.5 flex items-center gap-3 font-mono text-[10px] text-white/45" title="Traffic sent through this subscription's nodes in Vanish">
      <span>Via Vanish</span>
      <span class="flex items-center"><ArrowDown :size="9" />{{ fmtBytes(local.down) }}</span>
      <span class="flex items-center"><ArrowUp :size="9" />{{ fmtBytes(local.up) }}</span>
    </div>
  </div>
</template>
