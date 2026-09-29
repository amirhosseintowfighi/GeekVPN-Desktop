import { platform as osPlatform } from "@tauri-apps/plugin-os";

export type DesktopOs = "windows" | "macos" | "linux";

/** True inside the Tauri shell; false in a plain browser (vite dev, tests). */
export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/**
 * The OS the app runs on. Outside Tauri (a browser preview) the user agent is
 * the best we have; the desktop OS decides the title bar's layout.
 */
export function currentOs(): DesktopOs {
  if (inTauri) {
    const p = osPlatform();
    if (p === "windows" || p === "macos") return p;
    return "linux";
  }
  const ua = typeof navigator === "undefined" ? "" : navigator.userAgent;
  if (/Mac/i.test(ua)) return "macos";
  if (/Win/i.test(ua)) return "windows";
  return "linux";
}
