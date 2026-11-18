import { describe, expect, it } from "vitest";
import { alreadyAdded, linkIn } from "./importLink";
import type { ServersView } from "./servers";

describe("linkIn", () => {
  it("offers configs and subscription links", () => {
    expect(linkIn("  vless://abc@host:443?security=reality#DE  ")).toBe("vless://abc@host:443?security=reality#DE");
    expect(linkIn("https://panel.example.com:2096/sub/abc123")).toBe("https://panel.example.com:2096/sub/abc123");
    expect(linkIn("vmess://a\nss://b")).toBe("vmess://a\nss://b");
  });
  it("ignores ordinary text and web pages", () => {
    expect(linkIn("")).toBeNull();
    expect(linkIn("سلام، این لینک vless:// است")).toBeNull();
    expect(linkIn("https://www.digikala.com/product/123")).toBeNull();
    expect(linkIn("ftp://x")).toBeNull();
  });
});

describe("alreadyAdded", () => {
  const view = { sources: [{ url: "https://p.example/sub/a", links: ["vless://one"] }] } as unknown as ServersView;
  it("matches subscriptions by url and configs by link", () => {
    expect(alreadyAdded("https://p.example/sub/a", view)).toBe(true);
    expect(alreadyAdded("vless://one", view)).toBe(true);
    expect(alreadyAdded("vless://one\nvless://two", view)).toBe(false);
    expect(alreadyAdded("vless://one", null)).toBe(false);
  });
});
