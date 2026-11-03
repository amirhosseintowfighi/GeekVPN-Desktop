import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { save } from "@tauri-apps/plugin-dialog";
import { useEffect, useState, type FormEvent } from "react";
import { Link } from "react-router";
import { Button, Segmented } from "../design-system/controls";
import { Icon } from "../design-system/Icon";
import { PageHeader } from "../design-system/layout";
import { errorText, qrCode, qrSave } from "../lib/auth";
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
      <span className="micro text-[9px] tracking-[0.2em]">{label}</span>
      <span className="text-[28px] font-extrabold leading-tight">
        {value} <span className="text-[13px] font-normal text-ink-2">{unit}</span>
      </span>
      <div className="h-[3px] overflow-hidden rounded-sm bg-white/16">
        <div className={`h-full ${pct < 20 ? "bg-warn" : "bg-white"}`} style={{ width: `${Math.max(0, Math.min(100, pct))}%` }} />
      </div>
    </div>
  );
}

type Account = Extract<Source, { kind: "account" }>;

/** One account service (Desktop-Services card). */
function ServiceCard({ s, selected, onSelect }: { s: Account; selected: boolean; onSelect: () => void }) {
  const days = daysLeft(s.expiresAt, Date.now());
  const [stateLabel, tone] = STATE_FA[s.state] ?? [s.state, "warn"];
  const remaining = s.quotaGib === null ? null : Math.max(0, s.quotaGib - s.usedGib);
  const soon = s.state === "active" && days !== null && days <= 3;
  return (
    <article className={`glass-milk flex flex-col gap-4 rounded-[26px] p-5 ${selected ? "outline-2 outline-white/70" : ""}`}>
      <div className="flex items-start gap-3">
        <button type="button" onClick={onSelect} aria-pressed={selected} className="flex min-w-0 flex-1 flex-col gap-0.5 text-start">
          <span className="truncate text-lg font-extrabold">{s.name}</span>
          <span dir="ltr" className="truncate text-right font-num text-[11px] text-ink-2">
            {s.subscriptionId.slice(0, 8)} · {s.categoryNameFa || (s.tier ? TIER_FA[s.tier] ?? s.tier : "link")}
          </span>
        </button>
        <span
          className={`flex h-6 shrink-0 items-center whitespace-nowrap rounded-full px-2.5 text-[11px] font-extrabold ${
            tone === "ok" && !soon ? "bg-white text-navy" : tone === "bad" ? "bg-bad-soft text-bad" : "border border-white/30 bg-white/12"
          }`}
        >
          {soon ? `${faDigits(days ?? 0)} روز مانده` : stateLabel}
        </span>
      </div>
      <div className="flex gap-[18px]">
        <Meter label="Time left" value={days === null ? "—" : faDigits(days)} unit="روز" pct={days === null ? 0 : (days / 30) * 100} />
        <Meter
          label="Data left"
          value={remaining === null ? "نامحدود" : faDigits(remaining.toFixed(1))}
          unit={remaining === null ? "" : "گیگ"}
          pct={remaining === null || !s.quotaGib ? 100 : (remaining / s.quotaGib) * 100}
        />
      </div>
      <div className="flex items-center justify-between gap-2">
        <span className="text-[11px] text-ink-2">{faDigits(s.links.length)} سرور</span>
        <span className="flex gap-2">
          <Link to={`/usage?service=${encodeURIComponent(s.subscriptionId)}`} className="glass-milk flex h-9 items-center rounded-full px-3.5 text-xs font-bold text-white no-underline">
            مصرف
          </Link>
          <Link to={`/shop?renew=${encodeURIComponent(s.subscriptionId)}`} className="flex h-9 items-center rounded-full bg-white px-4 text-[13px] font-extrabold text-navy no-underline">
            تمدید
          </Link>
        </span>
      </div>
    </article>
  );
}

/** «لینک و QR»: the chosen service's subscription link, to use on other devices. */
function SharePanel({ s }: { s: Account | null }) {
  const [qr, setQr] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [note, setNote] = useState<string | null>(null);
  const url = s?.url ?? null;
  useEffect(() => {
    setQr(null);
    setNote(null);
    if (url) qrCode(url).then(setQr, (e) => setNote(errorText(e)));
  }, [url]);
  const saveQr = async () => {
    if (!url) return;
    const path = await save({ title: "ذخیره‌ی QR", defaultPath: "geekvpn-subscription.svg", filters: [{ name: "SVG", extensions: ["svg"] }] });
    if (!path) return;
    try {
      await qrSave(url, path);
      setNote("QR ذخیره شد.");
    } catch (e) {
      setNote(errorText(e));
    }
  };
  return (
    <section className="glass-milk flex w-[300px] shrink-0 flex-col gap-3 self-start rounded-[30px] p-5">
      <span className="micro text-[9px]">Link &amp; QR</span>
      <span className="text-lg font-extrabold">{s ? s.name : "لینک اشتراک"}</span>
      {url ? (
        <>
          <div className="flex h-[168px] w-[168px] items-center justify-center self-center rounded-3xl bg-white">
            {qr && <img src={`data:image/svg+xml;utf8,${encodeURIComponent(qr)}`} alt={`کد QR ${s?.name ?? ""}`} width={144} height={144} />}
          </div>
          <span dir="ltr" className="truncate rounded-full border border-white/25 bg-white/7 px-3.5 py-2 font-num text-[11px] text-ink-2">
            {url}
          </span>
          <div className="flex gap-2">
            <Button
              icon={copied ? "check" : "copy"}
              height={38}
              onClick={() =>
                void writeText(url).then(() => {
                  setCopied(true);
                  setTimeout(() => setCopied(false), 2000);
                })
              }
            >
              {copied ? "کپی شد" : "کپی لینک"}
            </Button>
            <Button kind="soft" icon="dl" height={38} onClick={() => void saveQr()}>
              ذخیره‌ی QR
            </Button>
          </div>
          <span className="text-[11px] leading-[1.8] text-ink-2">این لینک را در اپ‌های دیگر یا روی دستگاه دیگری اضافه کن. همه از حجم همین سرویس مصرف می‌کنند.</span>
        </>
      ) : (
        <span className="text-xs leading-[1.8] text-ink-2">{s ? "این سرویس لینک اشتراک ندارد." : "یک سرویس را انتخاب کن."}</span>
      )}
      {note && <span className="text-xs text-ink-2">{note}</span>}
    </section>
  );
}

const SHOW = [
  { value: "active", label: "فعال" },
  { value: "all", label: "همه" },
] as const;

/** Desktop-Services: the account's services, links added by hand, and one service's link and QR. */
export function Services() {
  const { view, add, removeSource, refresh, error } = useTunnel();
  const [text, setText] = useState("");
  const [busy, setBusy] = useState(false);
  const [show, setShow] = useState<"active" | "all">("active");
  const [pick, setPick] = useState<string | null>(null);
  const account = (view?.sources ?? []).filter((s): s is Account => s.kind === "account");
  const others = (view?.sources ?? []).filter((s) => s.kind !== "account");
  const active = account.filter((s) => s.state === "active");
  const shown = show === "active" ? active : account;
  const chosen = account.find((s) => s.id === pick) ?? shown[0] ?? null;

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    if (!text.trim()) return;
    setBusy(true);
    if (await add(text)) setText("");
    setBusy(false);
  };

  return (
    <>
      <div className="flex min-w-0 flex-1 flex-col gap-[18px] overflow-y-auto pe-1 [scrollbar-width:thin]">
        <PageHeader
          micro="Services"
          title="سرویس‌های من"
          subtitle="سرویس‌های حساب خودکار با ربات همگام می‌شوند"
          actions={
            <>
              <Button kind="soft" icon="refresh" height={40} onClick={() => void refresh()}>
                بروزرسانی
              </Button>
              <Link to="/shop" className="flex h-10 items-center gap-2 rounded-full bg-white px-[18px] text-[13px] font-extrabold text-navy no-underline">
                <Icon name="bag" size={16} />
                خرید سرویس
              </Link>
            </>
          }
        />
        {error && <span role="alert" className="text-[13px] text-bad">{error}</span>}
        {account.length > 0 ? (
          <>
            <div className="w-[300px]">
              <Segmented
                label="کدام سرویس‌ها"
                height={40}
                options={SHOW.map((o) => ({ value: o.value, label: `${o.label} · ${faDigits(o.value === "active" ? active.length : account.length)}` }))}
                value={show}
                onChange={setShow}
              />
            </div>
            <div className="grid grid-cols-2 gap-4 max-[1250px]:grid-cols-1">
              {shown.map((s) => (
                <ServiceCard key={s.id} s={s} selected={chosen?.id === s.id} onSelect={() => setPick(s.id)} />
              ))}
            </div>
          </>
        ) : (
          <span className="text-[13px] text-ink-2">سرویسی از حساب نیامده است. از «فروشگاه» بخر یا وارد حسابت شو.</span>
        )}

        <div className="flex flex-col gap-1">
          <span className="text-lg font-extrabold">لینک‌های دستی</span>
          <div className="flex flex-col border-t border-hair">
            {others.map((s) => (
              <div key={s.id} className="flex items-center gap-3 border-b border-hair py-3">
                <span dir="ltr" className="w-[30px] shrink-0 text-right font-num text-[10px] tracking-[0.18em] text-ink-2">
                  {s.kind === "link" ? "SUB" : "CFG"}
                </span>
                <span className="flex min-w-0 flex-1 flex-col gap-0.5">
                  <span className="truncate text-[15px] font-bold">{s.name}</span>
                  <span className="text-xs text-ink-2">
                    {faDigits(s.links.length)} سرور{s.kind === "link" ? " · با هر بروزرسانی تازه می‌شود" : ""}
                  </span>
                </span>
                <button
                  type="button"
                  aria-label={`حذف ${s.name}`}
                  onClick={() => void removeSource(s.id)}
                  className="flex h-9 w-9 items-center justify-center rounded-full text-ink-2 hover:bg-bad-soft hover:text-bad"
                >
                  <Icon name="trash" size={17} />
                </button>
              </div>
            ))}
          </div>
          <form onSubmit={(e) => void submit(e)} className="mt-2 flex items-center gap-2.5">
            <label htmlFor="add-link" className="sr-only">
              لینک اشتراک یا کانفیگ
            </label>
            <input
              id="add-link"
              dir="ltr"
              value={text}
              onChange={(e) => setText(e.target.value)}
              placeholder="https://… یا vless:// vmess:// trojan:// ss://"
              className="h-11 min-w-0 flex-1 rounded-full border border-white/25 bg-white/7 px-4 font-num text-xs text-white outline-none placeholder:text-white/60 focus:border-white/60"
            />
            <Button type="submit" kind="soft" icon="plus" height={44} disabled={busy || !text.trim()}>
              {busy ? "در حال افزودن…" : "افزودن"}
            </Button>
          </form>
          <span className="pt-1 text-[11px] text-ink-2">لینک اشتراک یا کانفیگ کپی‌شده را بچسبان؛ با <span dir="ltr" className="font-num">Ctrl+V</span> هم کار می‌کند.</span>
        </div>
      </div>
      {account.length > 0 && <SharePanel s={chosen} />}
    </>
  );
}
