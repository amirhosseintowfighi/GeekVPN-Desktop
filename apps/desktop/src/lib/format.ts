/** Bytes per second as the design writes it: "2.48 MB/s", "312 KB/s". */
export function speed(bps: number): { value: string; unit: string } {
  if (bps >= 1024 * 1024) return { value: (bps / 1024 / 1024).toFixed(2), unit: "MB/s" };
  if (bps >= 1024) return { value: String(Math.round(bps / 1024)), unit: "KB/s" };
  return { value: String(Math.round(bps)), unit: "B/s" };
}

/** A running connection's age as HH:MM:SS. */
export function elapsed(sinceMs: number, nowMs: number): string {
  const s = Math.max(0, Math.floor((nowMs - sinceMs) / 1000));
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${pad(Math.floor(s / 3600))}:${pad(Math.floor((s % 3600) / 60))}:${pad(s % 60)}`;
}

/** Whole days until `iso`, never negative; null for no date. */
export function daysLeft(iso: string | null, nowMs: number): number | null {
  if (!iso) return null;
  return Math.max(0, Math.ceil((Date.parse(iso) - nowMs) / 86_400_000));
}

/** ISO 3166 alpha-2 guessed from a server name's flag emoji, else its first letters. */
export function countryCode(name: string): string {
  const flags = [...name].filter((c) => {
    const p = c.codePointAt(0) ?? 0;
    return p >= 0x1f1e6 && p <= 0x1f1ff;
  });
  if (flags.length >= 2) return flags.slice(0, 2).map((c) => String.fromCharCode((c.codePointAt(0) ?? 0) - 0x1f1e6 + 65)).join("");
  const letters = name.replace(/[^A-Za-z]/g, "");
  return (letters.slice(0, 2) || "··").toUpperCase();
}

/** The name without its flag emoji, which the badge already shows. */
export function plainName(name: string): string {
  return [...name]
    .filter((c) => {
      const p = c.codePointAt(0) ?? 0;
      return !(p >= 0x1f1e6 && p <= 0x1f1ff);
    })
    .join("")
    .trim();
}

/** A byte count as the Connections table writes it: "1.28 KB", "2.31 MB". */
export function bytes(n: number): string {
  if (n >= 1024 * 1024 * 1024) return `${(n / 1024 ** 3).toFixed(2)} GB`;
  if (n >= 1024 * 1024) return `${(n / 1024 / 1024).toFixed(2)} MB`;
  if (n >= 1024) return `${(n / 1024).toFixed(2)} KB`;
  return `${n} B`;
}

/** How long ago `iso` was, in Persian: «همین الان», «۵ دقیقه پیش». */
export function ago(iso: string, nowMs: number): string {
  const s = Math.max(0, Math.floor((nowMs - Date.parse(iso)) / 1000));
  if (!Number.isFinite(s) || s < 10) return "همین الان";
  if (s < 60) return `${s} ثانیه پیش`;
  if (s < 3600) return `${Math.floor(s / 60)} دقیقه پیش`;
  return `${Math.floor(s / 3600)} ساعت پیش`;
}
