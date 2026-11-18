import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { inTauri } from "./platform";

export interface LockView {
  enabled: boolean;
  locked: boolean;
  /** 0 = only when the app starts. */
  idleMinutes: number;
  waitSecs: number;
}

function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!inTauri) return Promise.reject("این کار فقط داخل برنامه‌ی GeekVPN انجام می‌شود.");
  return invoke<T>(cmd, args);
}

/** Persian and Arabic digits typed on a Persian keyboard, as ASCII. */
export function asciiDigits(s: string): string {
  return s.replace(/[۰-۹]/g, (d) => String(d.charCodeAt(0) - 0x06f0)).replace(/[٠-٩]/g, (d) => String(d.charCodeAt(0) - 0x0660));
}

export const pinOk = (pin: string) => /^\d{4,8}$/.test(asciiDigits(pin));

export const lock = {
  state: () => call<LockView>("lock_state"),
  setPin: (pin: string, current?: string) => call<LockView>("lock_set_pin", { pin: asciiDigits(pin), current: current ? asciiDigits(current) : null }),
  disable: (pin: string) => call<LockView>("lock_disable", { pin: asciiDigits(pin) }),
  setIdle: (minutes: number) => call<LockView>("lock_set_idle", { minutes }),
  now: () => call<LockView>("lock_now"),
  unlock: (pin: string) => call<LockView>("lock_unlock", { pin: asciiDigits(pin) }),
  forgot: () => call<LockView>("lock_forgot"),
  onChange: (f: (v: LockView) => void): Promise<UnlistenFn> => (inTauri ? listen<LockView>("lock://changed", (e) => f(e.payload)) : Promise.resolve(() => {})),
};
