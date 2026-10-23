import { invoke } from "@tauri-apps/api/core";

export interface VanishNode {
  id: string;
  name: string;
  protocol: string;
  server: string;
  port: number;
  latency: number | null;
  subscription: string | null;
}

export interface Settings {
  mode: "system_proxy" | "tun" | "manual";
  engine: "sing_box" | "xray";
  mixed_port: number;
  api_port: number;
  core_path: string | null;
  selected_node: string | null;
  ping_url: string;
}

export interface CoreStatus {
  running: boolean;
  node_id: string | null;
}

export interface ImportResult {
  added: number;
  errors: string[];
}

export const api = {
  listNodes: () => invoke<VanishNode[]>("list_nodes"),
  importText: (text: string) => invoke<ImportResult>("import_text", { text }),
  importSubscription: (url: string) => invoke<ImportResult>("import_subscription", { url }),
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
