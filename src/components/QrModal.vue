<script setup lang="ts">
import { onMounted, ref } from "vue";
import QRCode from "qrcode";
import Modal from "./Modal.vue";
import { store } from "../stores/app";

const img = ref("");
const failed = ref(false);
const copied = ref(false);

onMounted(async () => {
  try {
    // High-contrast on white so phone cameras read it on the dark UI.
    img.value = await QRCode.toDataURL(store.ui.qr!.text, { margin: 2, width: 300, errorCorrectionLevel: "L" });
  } catch {
    failed.value = true; // payload too large for a QR code
  }
});

async function copy() {
  try {
    await navigator.clipboard.writeText(store.ui.qr!.text);
    copied.value = true;
    setTimeout(() => (copied.value = false), 1500);
  } catch {
    /* clipboard unavailable */
  }
}
</script>

<template>
  <Modal :title="store.ui.qr!.title" @close="store.ui.qr = null">
    <div class="flex flex-col items-center gap-3">
      <img v-if="img" :src="img" alt="QR code" class="h-60 w-60 rounded-lg" />
      <p v-else-if="failed" class="text-xs text-white/60">Too long to fit in a QR code. Copy it instead.</p>
      <p class="max-h-16 w-full overflow-hidden break-all font-mono text-[10px] text-white/45">{{ store.ui.qr!.text }}</p>
      <button class="btn" @click="copy">{{ copied ? "Copied" : "Copy" }}</button>
    </div>
  </Modal>
</template>
