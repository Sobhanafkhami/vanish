import { computed, reactive } from "vue";
import { listen } from "@tauri-apps/api/event";
import { api, type CoreStatus, type Settings, type VanishNode } from "../lib/api";

const HISTORY = 60;

export const store = reactive({
  nodes: [] as VanishNode[],
  settings: null as Settings | null,
  status: { running: false, node_id: null } as CoreStatus,
  busy: false,
  error: "" as string,
  latency: null as number | null,
  up: 0,
  down: 0,
  upTotal: 0,
  downTotal: 0,
  upHistory: Array(HISTORY).fill(0) as number[],
  downHistory: Array(HISTORY).fill(0) as number[],
  logs: [] as string[],
});

export const selectedNode = computed(() => store.nodes.find((n) => n.id === store.settings?.selected_node) ?? null);

function push(arr: number[], v: number) {
  arr.push(v);
  if (arr.length > HISTORY) arr.shift();
}

export async function refreshNodes() {
  store.nodes = await api.listNodes();
}

export async function init() {
  [store.settings, store.status] = await Promise.all([api.getSettings(), api.coreStatus()]);
  await refreshNodes();

  await listen<CoreStatus>("core-state", (e) => {
    store.status = e.payload;
    if (!e.payload.running) {
      store.up = store.down = 0;
      store.latency = null;
    }
  });
  await listen<{ up: number; down: number; up_total: number; down_total: number }>("traffic", (e) => {
    store.up = e.payload.up;
    store.down = e.payload.down;
    store.upTotal = e.payload.up_total;
    store.downTotal = e.payload.down_total;
    push(store.upHistory, e.payload.up);
    push(store.downHistory, e.payload.down);
  });
  await listen<string>("core-log", (e) => {
    store.logs.push(e.payload);
    if (store.logs.length > 200) store.logs.shift();
  });
}

async function guarded<T>(fn: () => Promise<T>): Promise<T | undefined> {
  store.busy = true;
  store.error = "";
  try {
    return await fn();
  } catch (e) {
    store.error = String(e);
  } finally {
    store.busy = false;
  }
}

export async function toggleConnection() {
  await guarded(async () => {
    store.status = store.status.running ? await api.disconnect() : await api.connect();
    if (store.status.running) setTimeout(measureLatency, 600);
  });
}

export async function measureLatency() {
  try {
    store.latency = await api.httpPing();
    await refreshNodes();
  } catch (e) {
    store.latency = null;
    store.error = String(e);
  }
}

export async function selectNode(id: string) {
  await guarded(async () => {
    await api.selectNode(id);
    if (store.settings) store.settings.selected_node = id;
    if (store.status.running) {
      // Restart the core so the new node takes effect.
      await api.disconnect();
      store.status = await api.connect();
    }
  });
}

export async function importInput(text: string): Promise<string> {
  const t = text.trim();
  const res = await guarded(() => (/^https?:\/\//i.test(t) && !t.includes("\n") ? api.importSubscription(t) : api.importText(t)));
  await refreshNodes();
  store.settings = await api.getSettings(); // import may have auto-selected a node
  if (!res) return store.error;
  return `Imported ${res.added} node(s)` + (res.errors.length ? `, ${res.errors.length} skipped` : "");
}

export async function removeNode(id: string) {
  await guarded(() => api.removeNode(id));
  await refreshNodes();
}

export async function pingAll() {
  await guarded(() => api.tcpPingAll());
  await refreshNodes();
}

export async function setMode(mode: Settings["mode"]) {
  if (!store.settings || store.status.running) return;
  store.settings = { ...store.settings, mode };
  await api.setSettings(store.settings);
}
