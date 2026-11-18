import jsQR from "jsqr";
import type { ServersView } from "./servers";

const PREFIXES = ["vless://", "vmess://", "trojan://", "ss://", "wireguard://", "wg://", "https://", "http://"];

/**
 * The link to offer from copied text: a config, or an https subscription.
 * Plain web addresses are too common in a clipboard, so an http(s) link
 * only counts when it looks like a subscription.
 */
export function linkIn(text: string): string | null {
  const t = text.trim();
  // A WireGuard config file, as WGDashboard and most panels hand it out.
  if (/^\[Interface\]/i.test(t) && /PrivateKey\s*=/i.test(t) && /\[Peer\]/i.test(t) && t.length <= 8192) return t;
  if (!t || t.length > 8192 || /\s/.test(t.split("\n")[0] ?? "")) return null;
  const first = t.split(/\r?\n/)[0]!.trim();
  if (!PREFIXES.some((p) => first.toLowerCase().startsWith(p))) return null;
  if (/^https?:\/\//i.test(first) && !/(sub|subscription|link|token|\/s\/|\/api\/v\d)/i.test(first)) return null;
  // Several configs pasted at once stay together.
  return /^https?:\/\//i.test(first) ? first : t;
}

/** Whether every config (or the subscription) is already in the list. */
export function alreadyAdded(link: string, view: ServersView | null): boolean {
  if (!view) return false;
  const lines = link.split(/\r?\n/).map((l) => l.trim()).filter(Boolean);
  return lines.every((l) => view.sources.some((s) => s.url === l || s.links.includes(l)));
}

/** The text of the QR code in an image, or null. */
export async function qrFromImage(blob: Blob): Promise<string | null> {
  const bitmap = await createImageBitmap(blob);
  // Big screenshots are scaled down: jsQR is quick on ~1000px and finds codes at that size.
  const scale = Math.min(1, 1200 / Math.max(bitmap.width, bitmap.height));
  const w = Math.max(1, Math.round(bitmap.width * scale));
  const h = Math.max(1, Math.round(bitmap.height * scale));
  const canvas = document.createElement("canvas");
  canvas.width = w;
  canvas.height = h;
  const ctx = canvas.getContext("2d");
  if (!ctx) return null;
  ctx.drawImage(bitmap, 0, 0, w, h);
  const data = ctx.getImageData(0, 0, w, h);
  const code = jsQR(data.data, w, h, { inversionAttempts: "attemptBoth" });
  return code?.data?.trim() || null;
}
