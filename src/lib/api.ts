import { invoke } from "@tauri-apps/api/core";

export interface VanishNode {
  id: string;
  name: string;
  protocol: string;
  server: string;
  port: number;
  latency: number | null;
  /** Owning subscription id; null for manually added nodes. */
  subscription: string | null;
  link: string | null;
  /** Lifetime bytes through this node. */
  up_total: number;
  down_total: number;
  /** Real HTTP delay in ms; -1 = failed, null = never tested. */
  delay: number | null;
}

export interface Subscription {
  id: string;
  name: string;
  url: string;
  updated_at: number | null;
  upload: number | null;
  download: number | null;
  total: number | null;
  expire: number | null;
}

export interface ExitInfo {
  ip: string;
  country: string;
  country_code: string;
  region: string;
  city: string;
  zip: string;
  host: string;
  isp: string;
  timezone: string;
}

export interface SpeedResult {
  via_tunnel: boolean;
  ping_ms: number;
  jitter_ms: number;
  download_mbps: number;
  upload_mbps: number;
}

export interface Check {
  label: string;
  status: "ok" | "warn" | "fail" | "info";
  detail: string;
}

export interface Diagnostics {
  connected: boolean;
  checks: Check[];
}

export interface RunningApp {
  name: string;
  title: string;
}

export interface UpdateInfo {
  version: string;
  notes: string | null;
}

export interface Settings {
  mode: "system_proxy" | "tun" | "manual";
  engine: "sing_box" | "xray";
  mixed_port: number;
  api_port: number;
  core_path: string | null;
  selected_node: string | null;
  ping_url: string;
  route_mode: "global" | "rule" | "direct";
  direct_domains: string[];
  direct_apps: string[];
  dns_server: string;
  autostart: boolean;
  connect_on_launch: boolean;
  auto_fallback: boolean;
  kill_switch: boolean;
  sub_refresh_hours: number;
}

export interface CoreStatus {
  running: boolean;
  node_id: string | null;
  blocked: boolean;
}

export interface ImportResult {
  added: number;
  errors: string[];
}

export const api = {
  listNodes: () => invoke<VanishNode[]>("list_nodes"),
  importText: (text: string) => invoke<ImportResult>("import_text", { text }),
  listSubscriptions: () => invoke<Subscription[]>("list_subscriptions"),
  addSubscription: (name: string, url: string) => invoke<ImportResult>("add_subscription", { name, url }),
  refreshSubscription: (id: string) => invoke<ImportResult>("refresh_subscription", { id }),
  renameSubscription: (id: string, name: string) => invoke<void>("rename_subscription", { id, name }),
  removeSubscription: (id: string) => invoke<void>("remove_subscription", { id }),
  pickFastest: () => invoke<string | null>("pick_fastest"),
  exitInfo: () => invoke<ExitInfo>("exit_info"),
  realDelay: (ids: string[]) => invoke<void>("real_delay", { ids }),
  speedTest: () => invoke<SpeedResult>("speed_test"),
  diagnostics: () => invoke<Diagnostics>("diagnostics"),
  resetSettings: () => invoke<Settings>("reset_settings"),
  listRunningApps: () => invoke<RunningApp[]>("list_running_apps"),
  pickAppExe: () => invoke<string | null>("pick_app_exe"),
  appVersion: () => invoke<string>("app_version"),
  checkUpdate: () => invoke<UpdateInfo | null>("check_update"),
  installUpdate: () => invoke<void>("install_update"),
  removeNode: (id: string) => invoke<void>("remove_node", { id }),
  selectNode: (id: string) => invoke<void>("select_node", { id }),
  getSettings: () => invoke<Settings>("get_settings"),
  setSettings: (settings: Settings) => invoke<void>("set_settings", { settings }),
  connect: () => invoke<CoreStatus>("connect"),
  disconnect: () => invoke<CoreStatus>("disconnect"),
  coreStatus: () => invoke<CoreStatus>("core_status"),
  tcpPing: (id: string) => invoke<number | null>("tcp_ping", { id }),
  tcpPingAll: () => invoke<void>("tcp_ping_all"),
  httpPing: () => invoke<number>("http_ping"),
};
