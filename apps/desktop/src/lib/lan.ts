import { invoke } from "@tauri-apps/api/core";
import { inTauri } from "./platform";

export interface LanShare {
  on: boolean;
  port: number;
  auth: boolean;
  user: string;
  pass: string;
}

export interface LanView {
  settings: LanShare;
  /** Whether the running connection carries it; null when not connected. */
  running: boolean | null;
  addresses: string[];
  link: string | null;
  qr: string | null;
}

export interface LanPatch {
  on?: boolean;
  port?: number;
  auth?: boolean;
  renew?: boolean;
}

function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!inTauri) return Promise.reject("این کار فقط داخل برنامه‌ی GeekVPN انجام می‌شود.");
  return invoke<T>(cmd, args);
}

export const lan = {
  view: () => call<LanView>("lan_view"),
  set: (patch: LanPatch) => call<LanView>("lan_set", { patch }),
};
