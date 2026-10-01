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

import { ago, bytes, countryCode, daysLeft, elapsed, plainName, speed } from "./format";

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

describe("connection figures", () => {
  it("bytes and age read as the Connections table writes them", () => {
    expect(bytes(610)).toBe("610 B");
    expect(bytes(1311)).toBe("1.28 KB");
    expect(bytes(2.31 * 1024 * 1024)).toBe("2.31 MB");
    const now = Date.parse("2026-09-30T10:00:00Z");
    expect(ago("2026-09-30T09:59:55Z", now)).toBe("همین الان");
    expect(ago("2026-09-30T09:55:00Z", now)).toBe("5 دقیقه پیش");
  });
});

import { durationLabel, planGrid, sizeFa, toman, usageSummary, type Storefront } from "./account";
import { percent } from "../pages/Referral";

describe("shop", () => {
  const plan = (planId: string, durationDays: number, quotaGib: number | null, isFeatured = false) => ({
    planId,
    nameFa: planId,
    planType: "volume",
    durationDays,
    price: 1000,
    compareAtPrice: null,
    quotaGib,
    dailyQuotaGib: null,
    deviceLimit: 2,
    badgeFa: null,
    isFeatured,
    descriptionFa: null,
  });
  const product = (tier: string, plans: ReturnType<typeof plan>[]) => ({
    productId: tier,
    tier,
    nameFa: tier,
    taglineFa: null,
    descriptionFa: null,
    featuresFa: [],
    badgeFa: null,
    isFeatured: false,
    plans,
  });
  const store: Storefront = {
    categories: [
      { categoryId: "c", nameFa: "c", icon: null, products: [product("direct", [plan("a", 30, 40), plan("b", 30, null), plan("c", 90, 20), plan("d", 30, 10)]), product("tunnel", [])] },
      { categoryId: "e", nameFa: "e", icon: null, products: [product("elite", [plan("x", 30, 50)])] },
    ],
    walletBalance: 0,
    loyaltyTier: "bronze",
    isFirstPurchase: true,
  };

  it("offers only tiers that have plans, durations in order, volumes smallest first with unlimited last", () => {
    const g = planGrid(store);
    expect(g.tiers).toEqual(["direct", "elite"]);
    expect(g.durations("direct")).toEqual([30, 90]);
    expect(g.volumes("direct", 30).map((p) => p.planId)).toEqual(["d", "a", "b"]);
    expect(planGrid(null).tiers).toEqual([]);
  });

  it("writes money, durations and shares in Persian", () => {
    expect(toman(150000)).toBe("۱۵۰٬۰۰۰");
    expect(durationLabel(90)).toBe("۳ ماهه");
    expect(durationLabel(45)).toBe("۴۵ روزه");
    expect(percent(1000)).toBe("٪۱۰");
    expect(percent(250)).toBe("٪۲٫۵");
  });
});

describe("usage", () => {
  it("sums, averages and finds the busiest day", () => {
    const s = usageSummary([
      { day: "2026-09-28", bytes: 0 },
      { day: "2026-09-29", bytes: 3 * 1024 ** 3 },
      { day: "2026-09-30", bytes: 1024 ** 3 },
    ]);
    expect(s.total).toBe(4 * 1024 ** 3);
    expect(s.peak?.day).toBe("2026-09-29");
    expect(usageSummary([{ day: "x", bytes: 0 }]).peak).toBeNull();
  });

  it("sizes in Persian units", () => {
    expect(sizeFa(46.6 * 1024 ** 3)).toEqual({ value: "۴۶٫۶", unit: "گیگ" });
    expect(sizeFa(1.55 * 1024 ** 3)).toEqual({ value: "۱٫۵۵", unit: "گیگ" });
    expect(sizeFa(312 * 1024 ** 2)).toEqual({ value: "۳۱۲", unit: "مگ" });
    expect(sizeFa(0)).toEqual({ value: "۰", unit: "مگ" });
  });
});

import { noteLines } from "./update";

describe("update notes", () => {
  it("reads Markdown bullets and headings as plain lines", () => {
    expect(noteLines("## What's new\n- **Faster** scanner\n* Kill Switch fix\n\n1. Split tunnel on macOS")).toEqual([
      "What's new",
      "Faster scanner",
      "Kill Switch fix",
      "Split tunnel on macOS",
    ]);
  });
});
