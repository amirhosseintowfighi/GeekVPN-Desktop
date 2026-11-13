import { useEffect, useState } from "react";
import { Button, Segmented, Switch } from "../design-system/controls";
import { Group, PageHeader, Row } from "../design-system/layout";
import { field } from "./Payment";
import type { Tuning } from "../lib/servers";
import { useTunnel } from "../lib/TunnelContext";

const FRAGMENT_PRESETS = [
  { value: "soft", label: "ملایم", length: "100-200", interval: "10-20" },
  { value: "strong", label: "قوی", length: "10-30", interval: "5-10" },
  { value: "custom", label: "دلخواه", length: "", interval: "" },
] as const;

const DNS_PRESETS = [
  { value: "1.1.1.1", label: "Cloudflare" },
  { value: "8.8.8.8", label: "Google" },
  { value: "9.9.9.9", label: "Quad9" },
  { value: "custom", label: "دلخواه" },
] as const;

/** "100-200" or "50", as the Rust side accepts it. */
function rangeOk(s: string, max: number): boolean {
  const n = (x: string) => /^\d+$/.test(x.trim()) && +x >= 1 && +x <= max;
  const [a, b] = s.split("-");
  return b === undefined ? n(a ?? "") : n(a ?? "") && n(b) && +a! <= +b;
}

function ipOk(s: string): boolean {
  const v = s.trim();
  return /^(\d{1,3})(\.\d{1,3}){3}$/.test(v) ? v.split(".").every((p) => +p <= 255) : /^[0-9a-f:]+$/i.test(v) && v.includes(":");
}

/** «تنظیمات پیشرفته»: Fragment, Mux, DNS and IPv6, as the design's Settings promised. */
export function Advanced() {
  const { view, set, state } = useTunnel();
  const t = view?.tuning;
  const [len, setLen] = useState("");
  const [gap, setGap] = useState("");
  const [dns, setDns] = useState("");
  useEffect(() => {
    if (!t) return;
    setLen(t.fragmentLength);
    setGap(t.fragmentInterval);
    setDns(t.dns);
  }, [t]);
  if (!t) return null;

  const save = (patch: Partial<Tuning>) => void set({ tuning: { ...t, ...patch } });
  const preset = FRAGMENT_PRESETS.find((p) => p.length === t.fragmentLength && p.interval === t.fragmentInterval)?.value ?? "custom";
  const dnsPreset = DNS_PRESETS.find((p) => p.value === t.dns)?.value ?? "custom";
  const connected = state.status === "on" || state.status === "connecting";
  const fragmentValid = rangeOk(len, 2000) && rangeOk(gap, 1000);

  return (
    <div className="flex min-w-0 flex-1 flex-col gap-[18px]">
      <PageHeader micro="Advanced" title="تنظیمات پیشرفته" subtitle={connected ? "تنظیمات ← پیشرفته · برای اعمال، یک بار قطع و وصل کن" : "تنظیمات ← پیشرفته · از اتصال بعدی اعمال می‌شود"} />
      <div className="flex min-h-0 flex-1 gap-10 overflow-y-auto pb-2 pe-1 [scrollbar-width:thin] max-[1100px]:flex-col max-[1100px]:gap-5">
        <div className="flex min-w-0 flex-1 basis-0 flex-col gap-4">
          <Group micro="Fragment">
            <Row
              title="تکه کردن TLS (Fragment)"
              hint="اولین بسته‌ی اتصال TLS تکه‌تکه فرستاده می‌شود تا فیلتر نام سرور را نبیند. وقتی سرور TLS یا Reality «وصل نمی‌شود»، روشنش کن."
              trailing={<Switch label="Fragment" checked={t.fragment} onChange={(v) => save({ fragment: v })} />}
            />
            {t.fragment && (
              <div className="flex flex-col gap-3 py-3">
                <Segmented
                  label="شدت Fragment"
                  height={40}
                  options={FRAGMENT_PRESETS.map((p) => ({ value: p.value, label: p.label }))}
                  value={preset}
                  onChange={(v) => {
                    const p = FRAGMENT_PRESETS.find((x) => x.value === v)!;
                    if (p.value !== "custom") save({ fragmentLength: p.length, fragmentInterval: p.interval });
                    else {
                      setLen(t.fragmentLength);
                      setGap(t.fragmentInterval);
                    }
                  }}
                />
                <div className="flex items-end gap-2.5">
                  <label className="flex min-w-0 flex-1 flex-col gap-1.5 text-xs text-ink-2">
                    اندازه‌ی هر تکه (بایت)
                    <input dir="ltr" value={len} onChange={(e) => setLen(e.target.value)} placeholder="100-200" className={`${field} font-num`} />
                  </label>
                  <label className="flex min-w-0 flex-1 flex-col gap-1.5 text-xs text-ink-2">
                    فاصله (میلی‌ثانیه)
                    <input dir="ltr" value={gap} onChange={(e) => setGap(e.target.value)} placeholder="10-20" className={`${field} font-num`} />
                  </label>
                  <Button kind="soft" height={46} disabled={!fragmentValid || (len === t.fragmentLength && gap === t.fragmentInterval)} onClick={() => save({ fragmentLength: len.trim(), fragmentInterval: gap.trim() })}>
                    ذخیره
                  </Button>
                </div>
                {!fragmentValid && <span className="text-[11px] text-bad">به شکل «۱۰۰-۲۰۰» یا یک عدد بنویس (اندازه تا ۲۰۰۰، فاصله تا ۱۰۰۰).</span>}
                <span className="text-[11px] leading-[1.8] text-ink-2">«قوی» برای فیلترهای سخت‌تر است ولی اتصال را کمی کندتر برقرار می‌کند. تست تأخیر هم با همین تنظیم انجام می‌شود.</span>
              </div>
            )}
          </Group>
          <Group micro="Mux">
            <Row
              title="Mux (چند اتصال در یکی)"
              hint="برای اینترنت‌های پرتأخیر صفحه‌ها را سریع‌تر باز می‌کند. بعضی سرورها قبولش نمی‌کنند؛ اگر بعد از روشن کردن وصل نشد، خاموشش کن. روی XTLS Vision، XHTTP و gRPC اعمال نمی‌شود."
              trailing={<Switch label="Mux" checked={t.mux} onChange={(v) => save({ mux: v })} />}
            />
          </Group>
        </div>

        <div className="flex min-w-0 flex-1 basis-0 flex-col gap-4">
          <div className="flex flex-col gap-2">
            <span className="micro text-[9px] tracking-[0.22em]">DNS</span>
            <span className="text-sm font-bold">سرور DNS از داخل تونل</span>
            <span className="text-[11px] leading-[1.8] text-ink-2">نام سایت‌هایی که از VPN می‌روند با این سرور و رمزشده (DNS over HTTPS) پیدا می‌شود. در حالت TUN اعمال می‌شود؛ در پروکسی سیستم، خود سرور VPN نام‌ها را پیدا می‌کند.</span>
            <Segmented
              label="سرور DNS"
              height={42}
              options={DNS_PRESETS.map((p) => ({ value: p.value, label: p.label }))}
              value={dnsPreset}
              onChange={(v) => (v === "custom" ? setDns(dnsPreset === "custom" ? t.dns : "") : save({ dns: v }))}
            />
            {(dnsPreset === "custom" || !DNS_PRESETS.some((p) => p.value === dns)) && (
              <div className="flex items-end gap-2.5">
                <label className="flex min-w-0 flex-1 flex-col gap-1.5 text-xs text-ink-2">
                  آدرس IP سرور DNS
                  <input dir="ltr" value={dns} onChange={(e) => setDns(e.target.value)} placeholder="94.140.14.14" className={`${field} font-num`} />
                </label>
                <Button kind="soft" height={46} disabled={!ipOk(dns) || dns.trim() === t.dns} onClick={() => save({ dns: dns.trim() })}>
                  ذخیره
                </Button>
              </div>
            )}
          </div>
          <Group micro="IPv6">
            <Row
              title="مسدود کردن IPv6"
              hint="در حالت TUN هیچ ترافیک IPv6ای بیرون نمی‌رود و نام‌ها فقط به IPv4 پیدا می‌شوند؛ جلوی نشت IP واقعی روی شبکه‌های IPv6 را می‌گیرد."
              trailing={<Switch label="مسدود کردن IPv6" checked={t.blockIpv6} onChange={(v) => save({ blockIpv6: v })} />}
            />
          </Group>
          <Button
            kind="soft"
            height={40}
            className="self-start"
            onClick={() => void set({ tuning: { fragment: false, fragmentLength: "100-200", fragmentInterval: "10-20", mux: false, dns: "1.1.1.1", blockIpv6: false } })}
          >
            بازگشت به پیش‌فرض
          </Button>
        </div>
      </div>
    </div>
  );
}
