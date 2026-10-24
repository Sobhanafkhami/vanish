import { computed, reactive } from "vue";
import { listen } from "@tauri-apps/api/event";
import { api, type CoreStatus, type ExitInfo, type Settings, type Subscription, type VanishNode } from "../lib/api";

const HISTORY = 60;

export const store = reactive({
  nodes: [] as VanishNode[],
  subs: [] as Subscription[],
  /** "all", "manual" (nodes without a subscription) or a subscription id. */
  activeTab: "all" as string,
  settings: null as Settings | null,
  status: { running: false, node_id: null, blocked: false } as CoreStatus,
  busy: false,
  error: "" as string,
  notice: "" as string,
  latency: null as number | null,
  exit: null as ExitInfo | null,
  up: 0,
  down: 0,
  upTotal: 0,
  downTotal: 0,
  upHistory: Array(HISTORY).fill(0) as number[],
  downHistory: Array(HISTORY).fill(0) as number[],
  logs: [] as string[],
  ui: { settings: false, logs: false, qr: null as { title: string; text: string } | null },
});

export const selectedNode = computed(() => store.nodes.find((n) => n.id === store.settings?.selected_node) ?? null);

function push(arr: number[], v: number) {
  arr.push(v);
  if (arr.length > HISTORY) arr.shift();
}

let noticeTimer: ReturnType<typeof setTimeout> | undefined;
export function notify(text: string) {
  store.notice = text;
  clearTimeout(noticeTimer);
  noticeTimer = setTimeout(() => (store.notice = ""), 5000);
}

export async function refreshNodes() {
  [store.nodes, store.subs] = await Promise.all([api.listNodes(), api.listSubscriptions()]);
  // A deleted subscription must not leave the user staring at a tab that no longer exists.
  if (store.activeTab !== "all" && store.activeTab !== "manual" && !store.subs.some((s) => s.id === store.activeTab)) {
    store.activeTab = "all";
  }
}

export async function init() {
  [store.settings, store.status] = await Promise.all([api.getSettings(), api.coreStatus()]);
  await refreshNodes();

  await listen<CoreStatus>("core-state", (e) => {
    store.status = e.payload;
    if (!e.payload.running) {
      store.up = store.down = 0;
      store.latency = null;
      store.exit = null;
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
    if (store.logs.length > 300) store.logs.shift();
  });
  await listen("nodes-changed", async () => {
    await refreshNodes();
    store.settings = await api.getSettings();
  });
  await listen<string>("fallback", async (e) => {
    notify(`Node unreachable — switched to ${e.payload}`);
    store.settings = await api.getSettings();
    store.status = await api.coreStatus();
    setTimeout(measureLatency, 600);
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
    return;
  }
  try {
    store.exit = await api.exitInfo();
  } catch {
    store.exit = null; // the IP lookup service is best-effort
  }
}

async function reconnectIfRunning() {
  if (store.status.running) {
    // Restart the core so the new node takes effect.
    await api.disconnect();
    store.status = await api.connect();
    setTimeout(measureLatency, 600);
  }
}

export async function selectNode(id: string) {
  await guarded(async () => {
    await api.selectNode(id);
    if (store.settings) store.settings.selected_node = id;
    await reconnectIfRunning();
  });
}

/** TCP-pings everything, picks the quickest reachable node and (re)connects to it if already connected. */
export async function connectFastest() {
  await guarded(async () => {
    const id = await api.pickFastest();
    await refreshNodes();
    store.settings = await api.getSettings();
    if (!id) throw "no node answered the TCP ping";
    const name = store.nodes.find((n) => n.id === id)?.name ?? "";
    notify(`Fastest: ${name}`);
    await reconnectIfRunning();
  });
}

/** Plain links / JSON go in as manual nodes. */
export async function importLinks(text: string): Promise<string> {
  const res = await guarded(() => api.importText(text.trim()));
  await refreshNodes();
  store.settings = await api.getSettings(); // import may have auto-selected a node
  if (!res) return store.error;
  store.activeTab = "manual";
  return `Imported ${res.added} node(s)` + (res.errors.length ? `, ${res.errors.length} skipped` : "");
}

export async function addSubscription(name: string, url: string): Promise<string> {
  const res = await guarded(() => api.addSubscription(name, url));
  await refreshNodes();
  store.settings = await api.getSettings();
  if (!res) return store.error;
  const sub = store.subs.find((s) => s.url === url.trim());
  if (sub) store.activeTab = sub.id;
  return `Imported ${res.added} node(s)` + (res.errors.length ? `, ${res.errors.length} skipped` : "");
}

export async function refreshSubscription(id: string) {
  const res = await guarded(() => api.refreshSubscription(id));
  await refreshNodes();
  if (res) notify(`Updated: ${res.added} node(s)`);
}

export async function renameSubscription(id: string, name: string) {
  await guarded(() => api.renameSubscription(id, name));
  await refreshNodes();
}

export async function removeSubscription(id: string) {
  await guarded(() => api.removeSubscription(id));
  await refreshNodes();
  store.settings = await api.getSettings();
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
  await updateSettings({ mode });
}

/** Persists a settings patch. Routing/DNS changes need a reconnect to take effect. */
export async function updateSettings(patch: Partial<Settings>) {
  if (!store.settings) return;
  const next = { ...store.settings, ...patch };
  try {
    await api.setSettings(next);
    store.settings = next;
  } catch (e) {
    store.error = String(e);
    return;
  }
}
