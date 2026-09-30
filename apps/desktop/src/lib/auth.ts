import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { inTauri } from "./platform";

/** The customer, as the Rust side describes them. Tokens never reach the UI. */
export interface AppUser {
  id: string;
  telegramId: number;
  displayName: string;
  username: string | null;
  referralCode: string;
  photoUrl: string | null;
}

export interface AuthView {
  user: AppUser | null;
  storeError: string | null;
}

export interface LoginView {
  deepLink: string;
  qrSvg: string;
  expiresIn: number;
}

export type LoginStatus = { status: "approved" | "denied" | "expired" | "error"; message: string | null };

const OUTSIDE_APP = "این کار فقط داخل برنامه‌ی GeekVPN انجام می‌شود.";

/** Commands reject with a Persian message string (see src-tauri/src/auth.rs). */
function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!inTauri) return Promise.reject(OUTSIDE_APP);
  return invoke<T>(cmd, args);
}

export const auth = {
  state: () => (inTauri ? call<AuthView>("auth_state") : Promise.resolve<AuthView>({ user: null, storeError: null })),
  startTelegram: () => call<LoginView>("login_telegram_start"),
  reopenTelegram: () => call<void>("login_open_telegram"),
  cancel: () => call<void>("login_cancel"),
  password: (username: string, password: string) => call<AuthView>("login_password", { username, password }),
  logout: () => call<void>("logout"),
  onChanged: (f: (v: AuthView) => void): Promise<UnlistenFn> =>
    inTauri ? listen<AuthView>("auth://changed", (e) => f(e.payload)) : Promise.resolve(() => {}),
  onLoginStatus: (f: (s: LoginStatus) => void): Promise<UnlistenFn> =>
    inTauri ? listen<LoginStatus>("login://status", (e) => f(e.payload)) : Promise.resolve(() => {}),
};

/** A rejected command's message, whatever shape it arrived in. */
export function errorText(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  return "کاری که خواستی انجام نشد. دوباره امتحان کن.";
}

const GUEST_KEY = "geek.guest";

/** «شروع سریع بدون ثبت‌نام»: skip the sign-in screen on this computer. */
export const guest = {
  get(): boolean {
    try {
      return localStorage.getItem(GUEST_KEY) === "1";
    } catch {
      return false;
    }
  },
  set(on: boolean): void {
    try {
      if (on) localStorage.setItem(GUEST_KEY, "1");
      else localStorage.removeItem(GUEST_KEY);
    } catch {
      // Without storage the choice lasts for this run only, which is fine.
    }
  },
};
