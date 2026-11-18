import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import { inTauri } from "./platform";

export interface TimerView {
  deadlineMs: number | null;
}

function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!inTauri) return Promise.reject("این کار فقط داخل برنامه‌ی GeekVPN انجام می‌شود.");
  return invoke<T>(cmd, args);
}

export const timer = {
  get: () => call<TimerView>("timer_get"),
  /** null cancels. */
  set: (minutes: number | null) => call<TimerView>("timer_set", { minutes }),
  onChange: (f: (v: TimerView) => void): Promise<UnlistenFn> => (inTauri ? listen<TimerView>("timer://changed", (e) => f(e.payload)) : Promise.resolve(() => {})),
};

/** The auto-disconnect deadline, kept in step with the app. */
export function useAutoOff(): number | null {
  const [deadline, setDeadline] = useState<number | null>(null);
  useEffect(() => {
    void timer.get().then((v) => setDeadline(v.deadlineMs), () => {});
    const off = timer.onChange((v) => setDeadline(v.deadlineMs));
    return () => void off.then((f) => f());
  }, []);
  return deadline;
}

/** "1:05:09" or "5:09" until `deadline`. */
export function countdown(deadline: number, now: number): string {
  const s = Math.max(0, Math.round((deadline - now) / 1000));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = String(s % 60).padStart(2, "0");
  return h > 0 ? `${h}:${String(m).padStart(2, "0")}:${sec}` : `${m}:${sec}`;
}
