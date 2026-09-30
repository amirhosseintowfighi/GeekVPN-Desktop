import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { useState, type FormEvent } from "react";
import { Link } from "react-router";
import { Icon } from "../design-system/Icon";
import { PageHeader } from "../design-system/layout";
import { faDigits } from "../lib/fa";
import { daysLeft } from "../lib/format";
import type { Source } from "../lib/servers";
import { useTunnel } from "../lib/TunnelContext";

const STATE_FA: Record<string, [string, "ok" | "warn" | "bad"]> = {
  active: ["فعال", "ok"],
  expired: ["منقضی", "bad"],
  exhausted: ["حجم تمام شده", "bad"],
  suspended: ["معلق", "warn"],
  revoked: ["لغو شده", "bad"],
};
const TIER_FA: Record<string, string> = { direct: "مستقیم", tunnel: "تونل", elite: "ویژه" };

function Meter({ label, value, unit, pct }: { label: string; value: string; unit: string; pct: number }) {
  return (
    <div className="flex flex-1 basis-0 flex-col gap-1">
      <span className="text-[11px] text-ink-2">{label}</span>
      <span className="text-[28px] font-extrabold leading-tight">
        {value} <span className="text-[13px] text-ink-2">{unit}</span>
      </span>
      <div className="h-1.5 rounded-[3px] bg-hair">
        <div className={`h-full rounded-[3px] ${pct < 20 ? "bg-warn" : "bg-link"}`} style={{ width: `${Math.max(0, Math.min(100, pct))}%` }} />
      </div>
    </div>
  );
}

/** One account service (Desktop-Services card). */
function ServiceCard({ s }: { s: Extract<Source, { kind: "account" }> }) {
  const [copied, setCopied] = useState(false);
  const days = daysLeft(s.expiresAt, Date.now());
  const [stateLabel, tone] = STATE_FA[s.state] ?? [s.state, "warn"];
  const remaining = s.quotaGib === null ? null : Math.max(0, s.quotaGib - s.usedGib);
  const toneCls = tone === "ok" ? "bg-ok-soft text-ok" : tone === "bad" ? "bg-bad-soft text-bad" : "bg-warn-soft text-warn";
  return (
    <article className="glass-milk flex-1 basis-0 overflow-hidden rounded-3xl">
      <div className="flex flex-col gap-4 p-4">
        <div className="flex items-center gap-3">
          <span className="flex h-12 w-12 items-center justify-center rounded-[15px] bg-logo">
            <Icon name="shield" size={24} color="#FFFFFF" />
          </span>
          <span className="flex min-w-0 flex-1 flex-col gap-0.5">
            <span className="truncate text-base font-extrabold">{s.name}</span>
            <span className="text-[11px] text-ink-2">
              {s.tier ? TIER_FA[s.tier] ?? s.tier : "لینک"} · {faDigits(s.links.length)} سرور
            </span>
          </span>
          <span className={`flex h-[26px] items-center gap-1.5 rounded-[9px] px-2.5 text-xs font-extrabold ${toneCls}`}>
            <span className="h-[7px] w-[7px] rounded-[2px] bg-current" />
            {stateLabel}
          </span>
        </div>
        <div className="flex gap-[18px]">
          <Meter label="زمان باقی‌مانده" value={days === null ? "—" : faDigits(days)} unit="روز" pct={days === null ? 0 : (days / 30) * 100} />
          <Meter
            label="حجم باقی‌مانده"
            value={remaining === null ? "نامحدود" : faDigits(remaining.toFixed(1))}
            unit={remaining === null ? "" : "گیگ"}
            pct={remaining === null || !s.quotaGib ? 100 : (remaining / s.quotaGib) * 100}
          />
        </div>
      </div>
      <div className="flex gap-2 border-t-2 border-dashed border-track bg-soft-button p-3">
        <Link to="/shop" className="flex h-11 flex-1 items-center justify-center gap-2 rounded-[14px] bg-action text-sm font-bold text-on-action no-underline">
          <Icon name="refresh" size={18} />
          تمدید
        </Link>
        <button
          type="button"
          aria-label="کپی لینک اشتراک"
          title="کپی لینک اشتراک — از حجم همین سرویس مصرف می‌کند"
          disabled={!s.url}
          onClick={() => {
            if (!s.url) return;
            void writeText(s.url).then(() => {
              setCopied(true);
              setTimeout(() => setCopied(false), 2000);
            });
          }}
          className="flex h-11 w-11 items-center justify-center rounded-[15px] bg-soft text-link"
        >
          <Icon name={copied ? "check" : "copy"} size={19} />
        </button>
      </div>
    </article>
  );
}

/** Desktop-Services: the account's services, then links added by hand. */
export function Services() {
  const { view, add, removeSource, refresh, error } = useTunnel();
  const [text, setText] = useState("");
  const [busy, setBusy] = useState(false);
  const account = (view?.sources ?? []).filter((s): s is Extract<Source, { kind: "account" }> => s.kind === "account");
  const others = (view?.sources ?? []).filter((s) => s.kind !== "account");

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    if (!text.trim()) return;
    setBusy(true);
    if (await add(text)) setText("");
    setBusy(false);
  };

  return (
    <div className="flex min-w-0 flex-1 flex-col gap-3.5 overflow-y-auto">
      <PageHeader
        title="سرویس‌های من"
        subtitle="سرویس‌های حساب خودکار با ربات همگام می‌شوند"
        actions={
          <button type="button" onClick={() => void refresh()} className="glass-clear flex h-[42px] items-center gap-2 rounded-[13px] px-3.5 text-[13px] font-bold">
            <Icon name="refresh" size={17} />
            بروزرسانی
          </button>
        }
      />
      {error && <span role="alert" className="rounded-xl bg-white/15 px-3 py-2 text-[13px]">{error}</span>}
      {account.length > 0 ? (
        <div className="grid grid-cols-2 gap-4">
          {account.map((s) => (
            <ServiceCard key={s.id} s={s} />
          ))}
        </div>
      ) : (
        <span className="text-[13px] opacity-90">سرویسی از حساب نیامده است. از «فروشگاه» بخر یا وارد حسابت شو.</span>
      )}

      <span className="mt-1 text-[13px] font-bold text-white/90">لینک‌های دستی</span>
      {others.map((s) => (
        <div key={s.id} className="glass-milk flex items-center gap-3 rounded-[22px] px-3.5 py-2.5">
          <span className="flex h-[38px] w-[38px] items-center justify-center rounded-xl bg-soft text-link">
            <Icon name="link" size={19} />
          </span>
          <span className="flex min-w-0 flex-1 flex-col">
            <span className="truncate text-sm font-bold">{s.name}</span>
            <span className="text-xs text-ink-2">
              {faDigits(s.links.length)} سرور{s.kind === "link" ? " · با هر بروزرسانی تازه می‌شود" : ""}
            </span>
          </span>
          <button
            type="button"
            aria-label={`حذف ${s.name}`}
            onClick={() => void removeSource(s.id)}
            className="flex h-[38px] w-[38px] items-center justify-center rounded-[13px] bg-bad-soft text-bad"
          >
            <Icon name="trash" size={18} />
          </button>
        </div>
      ))}
      <form onSubmit={(e) => void submit(e)} className="flex gap-2.5 rounded-[22px] border-[1.5px] border-dashed border-white/55 bg-white/6 p-3">
        <label htmlFor="add-link" className="sr-only">
          لینک اشتراک یا کانفیگ
        </label>
        <textarea
          id="add-link"
          dir="ltr"
          value={text}
          onChange={(e) => setText(e.target.value)}
          placeholder="https://… یا vless:// vmess:// trojan:// ss://"
          className="h-16 min-w-0 flex-1 resize-none rounded-xl border-none bg-white/12 p-2.5 font-num text-xs text-white outline-none placeholder:text-white/60"
        />
        <button type="submit" disabled={busy || !text.trim()} className="flex items-center gap-2 self-stretch rounded-[14px] bg-white px-4 text-sm font-bold text-[#062845] disabled:opacity-60">
          <Icon name="plus" size={18} />
          {busy ? "در حال افزودن…" : "افزودن"}
        </button>
      </form>
    </div>
  );
}
