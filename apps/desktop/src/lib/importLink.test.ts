import { describe, expect, it } from "vitest";
import { alreadyAdded, linkIn } from "./importLink";
import type { ServersView } from "./servers";

describe("linkIn", () => {
  it("offers configs and subscription links", () => {
    expect(linkIn("  vless://abc@host:443?security=reality#DE  ")).toBe("vless://abc@host:443?security=reality#DE");
    expect(linkIn("https://panel.example.com:2096/sub/abc123")).toBe("https://panel.example.com:2096/sub/abc123");
    expect(linkIn("vmess://a\nss://b")).toBe("vmess://a\nss://b");
    expect(linkIn("wireguard://key@1.2.3.4:51820?publickey=x&address=10.0.0.2/32#wg")).toContain("wireguard://");
    const conf = "[Interface]\nPrivateKey = abc=\nAddress = 10.0.0.2/32\n\n[Peer]\nPublicKey = def=\nEndpoint = wg.example:51820\n";
    expect(linkIn(conf)).toBe(conf.trim());
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

import { asciiDigits, pinOk } from "./lock";

describe("lock PIN", () => {
  it("accepts Persian digits as typed", () => {
    expect(asciiDigits("۱۲۳۴")).toBe("1234");
    expect(asciiDigits("٤٥٦٧")).toBe("4567");
    expect(pinOk("۱۲۳۴")).toBe(true);
    expect(pinOk("123")).toBe(false);
    expect(pinOk("12a4")).toBe(false);
  });
});

import { countdown } from "./timer";

describe("auto-off countdown", () => {
  it("shows hours only when needed", () => {
    expect(countdown(1000 + 3909_000, 1000)).toBe("1:05:09");
    expect(countdown(309_000, 0)).toBe("5:09");
    expect(countdown(0, 5000)).toBe("0:00");
  });
});
