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
