import { invoke } from "@tauri-apps/api/core";
import { inTauri } from "./platform";

/** «وصل شو» or «شبکه‌ی امن» (disconnect). */
export type NetAction = "connect" | "trusted";

export interface Network {
  key: string;
  /** "Wi-Fi", "کابل شبکه" or the adapter's name. */
  label: string;
}

export interface NetRule {
  key: string;
  name: string;
  action: NetAction;
}

export interface NetworksView {
  current: Network;
  rules: NetRule[];
  connectOnNew: boolean;
}

function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!inTauri) return Promise.reject("این کار فقط داخل برنامه‌ی GeekVPN انجام می‌شود.");
  return invoke<T>(cmd, args);
}

export const networks = {
  view: () => call<NetworksView>("networks_view"),
  /** `action: null` forgets the network. */
  setRule: (key: string, name: string, action: NetAction | null) => call<NetworksView>("network_rule_set", { key, name, action }),
  connectOnNew: (on: boolean) => call<NetworksView>("network_connect_on_new", { on }),
};

/** No network at all («نامشخص»): nothing to remember. */
export const OFFLINE = "other";
