import type { IconName } from "../design-system/Icon";

export interface NavItem {
  path: string;
  label: string;
  icon: IconName;
}

/** The sidebar, top to bottom (docs/ARCHITECTURE.md §10). */
export const NAV: readonly NavItem[] = [
  { path: "/", label: "خانه", icon: "home" },
  { path: "/servers", label: "سرورها", icon: "globe" },
  { path: "/services", label: "سرویس‌ها", icon: "shield" },
  { path: "/shop", label: "فروشگاه", icon: "bag" },
  { path: "/connections", label: "اتصالات", icon: "hub" },
  { path: "/tools", label: "ابزارها", icon: "tool" },
  { path: "/support", label: "پشتیبانی", icon: "chat" },
  { path: "/account", label: "حساب", icon: "user" },
  { path: "/settings", label: "تنظیمات", icon: "gear" },
];

/**
 * Ctrl+1…9 (⌘ on macOS) jumps to the nth sidebar item. Returns the path, or
 * null when the key is not one of ours. `code` is used so the shortcut works
 * on a Persian keyboard layout, where the digit keys type ۱…۹.
 */
export function shortcutTarget(e: Pick<KeyboardEvent, "code" | "ctrlKey" | "metaKey" | "altKey" | "shiftKey">, mac: boolean): string | null {
  const mod = mac ? e.metaKey && !e.ctrlKey : e.ctrlKey && !e.metaKey;
  if (!mod || e.altKey || e.shiftKey) return null;
  const m = /^Digit([1-9])$/.exec(e.code);
  if (!m) return null;
  return NAV[Number(m[1]) - 1]?.path ?? null;
}
