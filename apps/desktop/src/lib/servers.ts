import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { inTauri } from "./platform";

export type Route = "smart" | "global" | "direct";
/** «حالت اتصال»: the system proxy, or a TUN device through the helper. */
export type Mode = "proxy" | "tun";
export type AppMode = "off" | "bypass" | "only";
/** «Failover»: when a running connection moves to a better server. */
export type Failover = "off" | "lost" | "1000" | "2000" | "3000";

export interface AppRouting {
  mode: AppMode;
  paths: string[];
}

export interface ServerView {
  id: string;
  name: string;
  protocol: "vless" | "vmess" | "trojan" | "shadowsocks";
  address: string;
  port: number;
  network: string;
  security: string;
  sni: string;
  link: string;
  sourceId: string;
  favorite: boolean;
  /** Last real delay; ≤ 0 means no answer, null means never tested. */
  delayMs: number | null;
  cdn: boolean;
}

export type SourceKind =
  | {
      kind: "account";
      subscriptionId: string;
      tier: string | null;
      state: string;
      expiresAt: string | null;
      quotaGib: number | null;
      usedGib: number;
    }
  | { kind: "link" }
  | { kind: "manual" };

export type Source = {
  id: string;
  name: string;
  url: string | null;
  links: string[];
  updatedAt: number | null;
} & SourceKind;

export interface ServersView {
  sources: Source[];
  servers: ServerView[];
  selected: string | null;
  autoSelect: boolean;
  route: Route;
  sortByPing: boolean;
  autoConnect: boolean;
  closeToTray: boolean;
  shortcut: boolean;
  expiryAlert: boolean;
  failover: Failover;
  mode: Mode;
  killSwitch: boolean;
  strict: boolean;
  allowLan: boolean;
  apps: AppRouting;
}

export type TunnelState =
  | { status: "off" }
  /** `stage`: smart connect finding a clean IP, testing servers, or trying `attempt` of `of`. */
  | { status: "connecting"; serverId: string; attempt: number; of: number; stage: "findingIp" | "testing" | "connecting" }
  | {
      status: "on";
      serverId: string;
      serverName: string;
      sinceMs: number;
      mode: Mode;
      /** The local proxy ports; 0 in TUN mode. */
      httpPort: number;
      socksPort: number;
      delayMs: number;
      killSwitch: boolean;
      note: string | null;
    }
  /** `blocking`: the kill switch still holds the internet shut. */
  | { status: "failed"; message: string; blocking: boolean };

export interface TunnelStats {
  downBps: number;
  upBps: number;
  downTotal: number;
  upTotal: number;
}

export interface ServerSettings {
  favorite?: [string, boolean];
  selected?: string;
  autoSelect?: boolean;
  route?: Route;
  sortByPing?: boolean;
  autoConnect?: boolean;
  closeToTray?: boolean;
  shortcut?: boolean;
  expiryAlert?: boolean;
  failover?: Failover;
  mode?: Mode;
  killSwitch?: boolean;
  strict?: boolean;
  allowLan?: boolean;
  apps?: AppRouting;
}

const EMPTY: ServersView = {
  sources: [],
  servers: [],
  selected: null,
  autoSelect: true,
  route: "smart",
  sortByPing: false,
  autoConnect: false,
  closeToTray: true,
  shortcut: true,
  expiryAlert: true,
  failover: "2000",
  mode: "proxy",
  killSwitch: false,
  strict: false,
  allowLan: true,
  apps: { mode: "off", paths: [] },
};

function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!inTauri) return Promise.reject("این کار فقط داخل برنامه‌ی GeekVPN انجام می‌شود.");
  return invoke<T>(cmd, args);
}

function on<T>(event: string, f: (v: T) => void): Promise<UnlistenFn> {
  return inTauri ? listen<T>(event, (e) => f(e.payload)) : Promise.resolve(() => {});
}

export const servers = {
  state: () => (inTauri ? call<ServersView>("servers_state") : Promise.resolve(EMPTY)),
  refresh: () => call<ServersView>("servers_refresh"),
  add: (text: string) => call<ServersView>("servers_add", { text }),
  removeSource: (id: string) => call<ServersView>("servers_remove_source", { id }),
  set: (settings: ServerSettings) => call<ServersView>("servers_set", { settings }),
  test: () => call<ServersView>("servers_test"),
  onChanged: (f: () => void) => on<null>("servers://changed", () => f()),
  onDelay: (f: (r: { id: string; ms: number }) => void) => on("servers://delay", f),
};

export const tunnel = {
  state: () => (inTauri ? call<TunnelState>("tunnel_state") : Promise.resolve<TunnelState>({ status: "off" })),
  connect: () => call<void>("tunnel_connect"),
  disconnect: () => call<void>("tunnel_disconnect"),
  onState: (f: (s: TunnelState) => void) => on("tunnel://state", f),
  onStats: (f: (s: TunnelStats) => void) => on("tunnel://stats", f),
};

export type HelperStatus =
  | { state: "missing"; reason: string }
  | { state: "outdated"; version: string }
  | { state: "ready"; version: string; singBox: string };

export interface Connection {
  id: string;
  host: string;
  port: string;
  network: string;
  process: string;
  processPath: string;
  upload: number;
  download: number;
  /** RFC 3339. */
  start: string;
  route: "proxy" | "direct" | string;
}

export interface ConnectionsView {
  /** False unless connected in TUN mode. */
  available: boolean;
  connections: Connection[];
}

export interface Program {
  name: string;
  path: string;
}

export const system = {
  helperStatus: () => call<HelperStatus>("helper_status"),
  helperInstall: () => call<HelperStatus>("helper_install"),
  connections: () => call<ConnectionsView>("connections_list"),
  closeConnection: (id?: string) => call<void>("connections_close", { id: id ?? null }),
  programs: () => call<Program[]>("programs_running"),
};

export interface CleanIp {
  ip: string;
  port: number;
  pingMs: number;
  latencyMs: number;
  jitterMs: number;
  downloadKBps: number;
  colo: string | null;
}

export interface ScanView {
  network: { key: string; label: string };
  candidates: { id: string; name: string; sni: string }[];
  serverId: string | null;
  results: CleanIp[];
  scannedAt: number | null;
  inUse: string | null;
  behindCloudflare: boolean | null;
  downloadTest: boolean;
  running: boolean;
}

export type ScanEvent =
  | { kind: "progress"; tested: number; total: number; found: number }
  | { kind: "result"; result: CleanIp }
  | { kind: "finish"; error: string | null };

export interface SpeedResult {
  pingMs: number | null;
  jitterMs: number | null;
  downloadMbps: number | null;
  uploadMbps: number | null;
  throughVpn: boolean;
}

export const scanner = {
  state: (serverId?: string) => call<ScanView>("scan_state", { serverId: serverId ?? null }),
  start: (serverId: string, download: boolean) => call<string | null>("scan_start", { serverId, download }),
  stop: () => call<void>("scan_stop"),
  use: (serverId: string, ip: string | null) => call<void>("scan_use", { serverId, ip }),
  setDownload: (enabled: boolean) => call<void>("scan_set_download", { enabled }),
  onEvent: (f: (e: ScanEvent) => void) => on("scan://progress", f),
};

export const tools = {
  speedTest: () => call<SpeedResult>("speed_test"),
  cancelSpeed: () => call<void>("speed_cancel"),
  onSpeed: (f: (p: { phase: "ping" | "download" | "upload"; mbps: number }) => void) => on("speed://progress", f),
  coreLog: () => call<string>("core_log"),
};
