import { describe, expect, it } from "vitest";
import { faDigits } from "./fa";
import { NAV, shortcutTarget } from "./nav";
import { applyTheme, loadTheme } from "./theme";

const key = (code: string, mods: Partial<Record<"ctrlKey" | "metaKey" | "altKey" | "shiftKey", boolean>> = {}) => ({
  code,
  ctrlKey: false,
  metaKey: false,
  altKey: false,
  shiftKey: false,
  ...mods,
});

describe("faDigits", () => {
  it("turns digits and the decimal point Persian", () => {
    expect(faDigits("12.4")).toBe("۱۲٫۴");
    expect(faDigits(1011788123)).toBe("۱۰۱۱۷۸۸۱۲۳");
  });
});

describe("shortcutTarget", () => {
  it("maps Ctrl+n to the nth sidebar item", () => {
    expect(shortcutTarget(key("Digit1", { ctrlKey: true }), false)).toBe("/");
    expect(shortcutTarget(key("Digit9", { ctrlKey: true }), false)).toBe(NAV[8]!.path);
  });
  it("uses ⌘ on macOS and ignores Ctrl there", () => {
    expect(shortcutTarget(key("Digit2", { metaKey: true }), true)).toBe("/servers");
    expect(shortcutTarget(key("Digit2", { ctrlKey: true }), true)).toBeNull();
  });
  it("ignores other keys and extra modifiers", () => {
    expect(shortcutTarget(key("KeyA", { ctrlKey: true }), false)).toBeNull();
    expect(shortcutTarget(key("Digit1", { ctrlKey: true, shiftKey: true }), false)).toBeNull();
    expect(shortcutTarget(key("Digit1"), false)).toBeNull();
  });
});

describe("theme", () => {
  it("stores an explicit choice and forgets it for system", () => {
    const root = document.createElement("html");
    applyTheme("dark", root);
    expect(root.dataset.theme).toBe("dark");
    expect(loadTheme()).toBe("dark");
    applyTheme("system", root);
    expect(root.dataset.theme).toBeUndefined();
    expect(loadTheme()).toBe("system");
  });
});

import { countryCode, daysLeft, elapsed, plainName, speed } from "./format";

describe("format", () => {
  it("writes speeds the way the design does", () => {
    expect(speed(2.48 * 1024 * 1024)).toEqual({ value: "2.48", unit: "MB/s" });
    expect(speed(312 * 1024)).toEqual({ value: "312", unit: "KB/s" });
    expect(speed(0)).toEqual({ value: "0", unit: "B/s" });
  });
  it("counts connection time and days left", () => {
    expect(elapsed(0, (12 * 60 + 48) * 1000)).toBe("00:12:48");
    expect(daysLeft("2026-10-27T00:00:00Z", Date.parse("2026-09-29T00:00:00Z"))).toBe(28);
    expect(daysLeft(null, 0)).toBeNull();
  });
  it("reads the country from a flag emoji", () => {
    expect(countryCode("🇩🇪 Germany")).toBe("DE");
    expect(plainName("🇩🇪 Germany")).toBe("Germany");
    expect(countryCode("tunnel-1")).toBe("TU");
  });
});
