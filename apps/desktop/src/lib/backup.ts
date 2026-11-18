import { invoke } from "@tauri-apps/api/core";
import { inTauri } from "./platform";

export interface RestoreSummary {
  linksAdded: number;
  rules: number;
}

/** What a restore fails with when the file is sealed and no password was given. */
export const NEEDS_PASSWORD = "password-needed";

function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!inTauri) return Promise.reject("این کار فقط داخل برنامه‌ی GeekVPN انجام می‌شود.");
  return invoke<T>(cmd, args);
}

export const backup = {
  save: (path: string, password: string | null) => call<void>("backup_save", { path, password }),
  restore: (path: string, password: string | null) => call<RestoreSummary>("backup_restore", { path, password }),
};
