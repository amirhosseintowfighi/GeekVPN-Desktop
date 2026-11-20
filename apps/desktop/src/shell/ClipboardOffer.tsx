import { readText } from "@tauri-apps/plugin-clipboard-manager";
import { useEffect, useRef, useState } from "react";
import { useNavigate } from "react-router";
import { Button } from "../design-system/controls";
import { Icon } from "../design-system/Icon";
import { alreadyAdded, linkIn } from "../lib/importLink";
import { inTauri } from "../lib/platform";
import { useTunnel } from "../lib/TunnelContext";

/** «تشخیص لینک کپی‌شده»: on by default, a per-computer choice. */
export const CLIPBOARD_KEY = "geek.clipboardOffer";

export function clipboardOfferOn(): boolean {
  try {
    return localStorage.getItem(CLIPBOARD_KEY) !== "0";
  } catch {
    return true;
  }
}

export function setClipboardOffer(on: boolean) {
  try {
    localStorage.setItem(CLIPBOARD_KEY, on ? "1" : "0");
  } catch {
    // Without storage the default stays.
  }
}

const SEEN_KEY = "geek.clipboardSeen";

/** A short fingerprint, so what was offered is remembered without keeping the link. */
function fingerprint(text: string): string {
  let h = 5381;
  for (let i = 0; i < text.length; i++) h = ((h << 5) + h + text.charCodeAt(i)) | 0;
  return `${text.length}:${(h >>> 0).toString(36)}`;
}

function seenBefore(text: string): boolean {
  try {
    return (JSON.parse(localStorage.getItem(SEEN_KEY) ?? "[]") as string[]).includes(fingerprint(text));
  } catch {
    return false;
  }
}

function remember(text: string) {
  try {
    const list = (JSON.parse(localStorage.getItem(SEEN_KEY) ?? "[]") as string[]).filter((x) => x !== fingerprint(text));
    localStorage.setItem(SEEN_KEY, JSON.stringify([fingerprint(text), ...list].slice(0, 50)));
  } catch {
    // Without storage it is offered again next time, nothing worse.
  }
}

/**
 * A config or subscription link copied elsewhere (Telegram, a browser) is
 * offered when the window comes to the front, once per link; nothing is
 * added without a click, and the clipboard is only read on focus.
 */
export function ClipboardOffer() {
  const { view, add } = useTunnel();
  const navigate = useNavigate();
  const [link, setLink] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const seen = useRef(new Set<string>());
  const viewRef = useRef(view);
  viewRef.current = view;

  useEffect(() => {
    if (!inTauri) return;
    const look = () => {
      if (!clipboardOfferOn()) return;
      readText().then(
        (text) => {
          const l = linkIn(text ?? "");
          // Offered once, ever: added or dismissed, it does not come back.
          if (!l || seen.current.has(l) || seenBefore(l) || alreadyAdded(l, viewRef.current)) return;
          seen.current.add(l);
          remember(l);
          setLink(l);
        },
        () => {},
      );
    };
    look();
    window.addEventListener("focus", look);
    return () => window.removeEventListener("focus", look);
  }, []);

  if (!link) return null;
  const sub = /^https?:\/\//i.test(link);
  const count = sub ? 0 : link.split(/\r?\n/).filter((l) => l.trim()).length;
  const shown = link.split(/\r?\n/)[0]!.slice(0, 70);

  return (
    <div role="status" className="glass-milk absolute bottom-8 left-1/2 z-50 flex w-[min(560px,calc(100%-48px))] -translate-x-1/2 items-center gap-3 rounded-[22px] px-4 py-3 shadow-2xl">
      <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-white text-navy">
        <Icon name="link" size={18} />
      </span>
      <span className="flex min-w-0 flex-1 flex-col gap-0.5">
        <span className="text-[13px] font-extrabold">{sub ? "لینک اشتراک کپی کرده‌ای" : count > 1 ? `${count} کانفیگ کپی کرده‌ای` : "کانفیگ کپی کرده‌ای"}</span>
        <span dir="ltr" className="truncate text-start font-num text-[11px] text-ink-2">
          {shown}
          {link.length > shown.length ? "…" : ""}
        </span>
      </span>
      <Button
        kind="white"
        height={36}
        disabled={busy}
        onClick={async () => {
          setBusy(true);
          const ok = await add(link);
          setBusy(false);
          setLink(null);
          if (ok) void navigate("/servers");
        }}
      >
        {busy ? "در حال افزودن…" : "افزودن"}
      </Button>
      <button type="button" aria-label="نه، ممنون" onClick={() => setLink(null)} className="flex h-9 w-9 shrink-0 items-center justify-center rounded-full text-ink-2 hover:bg-white/10">
        <Icon name="x" size={16} />
      </button>
    </div>
  );
}
