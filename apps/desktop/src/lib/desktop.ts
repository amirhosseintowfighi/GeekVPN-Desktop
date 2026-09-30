import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { inTauri } from "./platform";

/**
 * "flyout" in the small panel beside the tray icon, "main" otherwise. In a
 * browser preview, `?window=flyout` shows the panel.
 */
export function windowLabel(): string {
  if (inTauri) return getCurrentWindow().label;
  return typeof location !== "undefined" && new URLSearchParams(location.search).get("window") === "flyout" ? "flyout" : "main";
}

export const desktop = {
  /** «اجرا با روشن شدن سیستم»: read, or set with `enabled`. */
  autostart: (enabled?: boolean) =>
    inTauri ? invoke<{ enabled: boolean }>("desktop_autostart", { enabled: enabled ?? null }) : Promise.resolve({ enabled: false }),
  /** The main window, optionally at a page. */
  open: (path?: string) => (inTauri ? invoke<void>("desktop_open", { path: path ?? null }) : Promise.resolve()),
  quit: () => (inTauri ? invoke<void>("desktop_quit") : Promise.resolve()),
  hideSelf: () => (inTauri ? getCurrentWindow().hide() : Promise.resolve()),
  onNavigate: (f: (path: string) => void): Promise<UnlistenFn> =>
    inTauri ? listen<string>("desktop://navigate", (e) => f(e.payload)) : Promise.resolve(() => {}),
};
