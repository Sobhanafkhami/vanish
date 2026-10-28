import { computed, reactive } from "vue";
import { listen } from "@tauri-apps/api/event";
import { errorText, friendlyError, t } from "../lib/i18n";
import {
  api,
  type CoreStatus,
  type Diagnostics,
  type ExitInfo,
  type Settings,
  type SpeedResult,
  type Subscription,
  type UpdateInfo,
  type VanishNode,
} from "../lib/api";

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
  errorHint: "" as string,
  errorRaw: "" as string,
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
  delayTesting: false,
  version: "",
  update: null as UpdateInfo | null,
  updating: false,
  speed: {
    running: false,
    phase: "" as "" | "ping" | "download" | "upload" | "done",
    live: 0,
    /** Raw readings of the current phase, for the live chart. */
    samples: [] as number[],
    /** Finished phases of the running test (ms for ping, Mbit/s otherwise). */
    partial: { ping: null as number | null, download: null as number | null, upload: null as number | null },
    result: null as SpeedResult | null,
    error: "",
  },
  diag: { running: false, result: null as Diagnostics | null, error: "" },
  ui: {
    settings: false,
    logs: false,
    tools: false,
    appPicker: false,
    qr: null as { title: string; text: string } | null,
  },
});

/** Settings whose value is baked into the running core's config. */
const RECONNECT_KEYS: (keyof Settings)[] = ["route_mode", "direct_domains", "direct_apps", "dns_server", "mixed_port", "api_port"];

export const selectedNode = computed(() => store.nodes.find((n) => n.id === store.settings?.selected_node) ?? null);

function push(arr: number[], v: number) {
  arr.push(v);
  if (arr.length > HISTORY) arr.shift();
}

export function fail(e: unknown) {
  const f = friendlyError(e);
  store.error = f.title;
  store.errorHint = f.hint;
  store.errorRaw = f.raw;
}

export function clearError() {
  store.error = store.errorHint = store.errorRaw = "";
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
  await listen<{ id: string; delay: number }>("node-delay", (e) => {
    const n = store.nodes.find((x) => x.id === e.payload.id);
    if (n) n.delay = e.payload.delay;
  });
  await listen("usage-changed", () => refreshNodes());
  await listen<{ phase: "ping" | "download" | "upload" | "done"; value: number; done: boolean }>("speedtest", (e) => {
    const { phase, value, done } = e.payload;
    if (phase === "done") return;
    const sp = store.speed;
    if (sp.phase !== phase) {
      sp.phase = phase;
      sp.samples = [];
      sp.live = 0;
    }
    if (done) {
      sp.partial[phase] = value;
      sp.live = phase === "ping" ? 0 : value;
    } else if (phase !== "ping") {
      sp.live = sp.live ? sp.live * 0.6 + value * 0.4 : value; // smooth the needle
      sp.samples.push(value);
      if (sp.samples.length > 60) sp.samples.shift();
    }
  });
  await listen<UpdateInfo>("update-available", (e) => (store.update = e.payload));
  api.appVersion().then((v) => (store.version = v));
  await listen("nodes-changed", async () => {
    await refreshNodes();
    store.settings = await api.getSettings();
  });
  await listen<string>("fallback", async (e) => {
    notify(t("Node unreachable — switched to {name}", { name: e.payload }));
    store.settings = await api.getSettings();
    store.status = await api.coreStatus();
    setTimeout(measureLatency, 600);
  });
}

async function guarded<T>(fn: () => Promise<T>): Promise<T | undefined> {
  store.busy = true;
  clearError();
  try {
    return await fn();
  } catch (e) {
    fail(e);
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
    fail(e);
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
    notify(t("Fastest: {name}", { name }));
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
  return t("Imported {n} node(s)", { n: res.added }) + (res.errors.length ? t(", {n} skipped", { n: res.errors.length }) : "");
}

export async function addSubscription(name: string, url: string): Promise<string> {
  const res = await guarded(() => api.addSubscription(name, url));
  await refreshNodes();
  store.settings = await api.getSettings();
  if (!res) return store.error;
  const sub = store.subs.find((s) => s.url === url.trim());
  if (sub) store.activeTab = sub.id;
  return t("Imported {n} node(s)", { n: res.added }) + (res.errors.length ? t(", {n} skipped", { n: res.errors.length }) : "");
}

export async function refreshSubscription(id: string) {
  const res = await guarded(() => api.refreshSubscription(id));
  await refreshNodes();
  if (res) notify(t("Updated: {n} node(s)", { n: res.added }));
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

/** Persists a settings patch and reconnects when the change affects the running core. */
export async function updateSettings(patch: Partial<Settings>) {
  if (!store.settings) return;
  const next = { ...store.settings, ...patch };
  try {
    await api.setSettings(next);
    store.settings = next;
  } catch (e) {
    fail(e);
    return;
  }
  // Routing, DNS and ports are part of the generated config: apply them to a running core right away.
  if (RECONNECT_KEYS.some((k) => k in patch)) await guarded(reconnectIfRunning);
}

/** HTTP(S) delay through each of `ids` (like URL test in Nekoray); results arrive live via `node-delay`. */
export async function realDelay(ids: string[]) {
  if (store.delayTesting || !ids.length) return;
  store.delayTesting = true;
  clearError();
  ids.forEach((id) => {
    const n = store.nodes.find((x) => x.id === id);
    if (n) n.delay = null;
  });
  try {
    await api.realDelay(ids);
  } catch (e) {
    fail(e);
  } finally {
    store.delayTesting = false;
    await refreshNodes();
  }
}

export async function runSpeedTest() {
  if (store.speed.running) return;
  Object.assign(store.speed, {
    running: true,
    phase: "ping",
    live: 0,
    samples: [],
    partial: { ping: null, download: null, upload: null },
    result: null,
    error: "",
  });
  try {
    const r = await api.speedTest();
    store.speed.result = r;
    store.speed.partial = { ping: r.ping_ms, download: r.download_mbps, upload: r.upload_mbps };
    store.speed.live = r.download_mbps;
  } catch (e) {
    store.speed.error = errorText(e);
  } finally {
    store.speed.running = false;
    store.speed.phase = "done";
  }
}

export async function runDiagnostics() {
  if (store.diag.running) return;
  Object.assign(store.diag, { running: true, result: null, error: "" });
  try {
    store.diag.result = await api.diagnostics();
  } catch (e) {
    store.diag.error = errorText(e);
  } finally {
    store.diag.running = false;
  }
}

export async function addDirectApp(name: string) {
  const n = name.trim();
  if (!n || !store.settings) return;
  if (store.settings.direct_apps.some((a) => a.toLowerCase() === n.toLowerCase())) return;
  await updateSettings({ direct_apps: [...store.settings.direct_apps, n] });
}

export async function resetSettings() {
  try {
    store.settings = await api.resetSettings();
    await reconnectIfRunning();
    notify(t("Settings restored to defaults"));
  } catch (e) {
    fail(e);
  }
}

/** Returns a short status line for the Settings page. */
export async function checkForUpdate(): Promise<string> {
  try {
    store.update = await api.checkUpdate();
    return store.update ? t("Version {v} is available", { v: store.update.version }) : t("You are on the latest version");
  } catch (e) {
    return t("Could not check for updates: {e}", { e: friendlyError(e).title });
  }
}

export async function installUpdate() {
  store.updating = true;
  try {
    await api.installUpdate(); // restarts the app on success
  } catch (e) {
    fail(e);
    store.updating = false;
  }
}
