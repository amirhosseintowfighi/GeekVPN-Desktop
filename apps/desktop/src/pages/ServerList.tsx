import { useEffect, useMemo, useRef, useState } from "react";
import { Link } from "react-router";
import { Icon } from "../design-system/Icon";
import { Button, Hair, SearchField, Segmented, Switch } from "../design-system/controls";
import { Card, CardTitle, PageHeader } from "../design-system/layout";
import { faDigits } from "../lib/fa";
import { countryCode, flagOf, plainName } from "../lib/format";
import { inTauri } from "../lib/platform";
import { scanner, type Failover, type ScanView, type ServerView, type Source } from "../lib/servers";
import { useTunnel } from "../lib/TunnelContext";

/** The sources that have servers, the one in use, and its servers. */
export function useActiveSource() {
  const { view } = useTunnel();
  const sources = (view?.sources ?? []).filter((s) => s.links.length > 0);
  const active = sources.find((s) => s.id === view?.activeSource) ?? sources[0] ?? null;
  const all = (view?.servers ?? []).filter((s) => s.sourceId === active?.id);
  // The panel's status lines go above the list, unless they are all it has.
  const real = all.filter((s) => !s.info);
  const servers = real.length ? real : all;
  const infos = real.length ? all.filter((s) => s.info) : [];
  return { sources, active, servers, infos };
}

/** «📊 24GB / 200GB · ⏳ ۲۸ روز»: what the subscription says about itself. */
export function SourceInfo() {
  const { infos } = useActiveSource();
  if (!infos.length) return null;
  return (
    <div className="flex flex-wrap gap-1.5" aria-label="وضعیت اشتراک">
      {infos.map((i) => (
        <span key={i.id} dir="auto" className="inline-flex h-7 max-w-full items-center truncate rounded-full border border-white/20 bg-white/6 px-2.5 text-[11px] font-bold text-ink-2">
          {i.name.trim()}
        </span>
      ))}
    </div>
  );
}

function matches(s: ServerView, q: string): boolean {
  const needle = q.trim().toLowerCase();
  return !needle || s.name.toLowerCase().includes(needle) || s.address.toLowerCase().includes(needle);
}

function latency(ms: number | null | undefined): string {
  return ms === null || ms === undefined ? "—" : ms > 0 ? `${ms} ms` : "×";
}

/** The server's flag from its name, or its letters when it has none. */
export function ServerMark({ name, className = "" }: { name: string; className?: string }) {
  const flag = flagOf(name);
  return flag ? (
    <span aria-hidden="true" className={`w-7 shrink-0 text-center text-[18px] leading-none ${className}`}>
      {flag}
    </span>
  ) : (
    <span aria-hidden="true" dir="ltr" className={`w-7 shrink-0 text-center font-num text-[10px] tracking-[0.12em] opacity-70 ${className}`}>
      {countryCode(name)}
    </span>
  );
}

/** The name as written: an English name reads left to right inside the Persian page. */
export function serverName(s: ServerView): string {
  return plainName(s.name) || s.address;
}

/** «کدام اشتراک»: pick the subscription whose servers are shown and used. */
export function SourcePicker({ wide }: { wide?: boolean }) {
  const { set } = useTunnel();
  const { sources, active } = useActiveSource();
  const [open, setOpen] = useState(false);
  const box = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!open) return;
    const close = (e: MouseEvent) => {
      if (!box.current?.contains(e.target as Node)) setOpen(false);
    };
    const esc = (e: KeyboardEvent) => e.key === "Escape" && setOpen(false);
    window.addEventListener("mousedown", close);
    window.addEventListener("keydown", esc);
    return () => {
      window.removeEventListener("mousedown", close);
      window.removeEventListener("keydown", esc);
    };
  }, [open]);
  if (!active) return null;
  const sub = (s: Source) => (s.kind === "account" ? s.categoryNameFa || "سرویس" : s.kind === "link" ? "لینک اشتراک" : "کانفیگ دستی");
  return (
    <div ref={box} className={`relative ${wide ? "w-[260px]" : "w-full"}`}>
      <button
        type="button"
        aria-haspopup="listbox"
        aria-expanded={open}
        onClick={() => setOpen(!open)}
        className="glass-milk flex h-11 w-full items-center gap-2.5 rounded-full ps-4 pe-3 text-start"
      >
        <span className="flex min-w-0 flex-1 flex-col leading-tight">
          <span className="truncate text-[13px] font-extrabold">{active.name}</span>
          <span className="truncate text-[10px] text-ink-2">
            {sub(active)} · {faDigits(active.links.length)} سرور
          </span>
        </span>
        {sources.length > 1 && (
          <span className={`flex text-ink-2 transition-transform ${open ? "rotate-90" : "-rotate-90"}`}>
            <Icon name="chev" size={16} />
          </span>
        )}
      </button>
      {open && sources.length > 1 && (
        <div role="listbox" aria-label="اشتراک" className="absolute inset-x-0 top-[calc(100%+6px)] z-30 flex flex-col gap-0.5 rounded-[22px] border border-white/25 bg-[#0b1440] p-1.5 shadow-[0_18px_40px_rgba(0,4,30,0.6)]">
          {sources.map((s) => {
            const on = s.id === active.id;
            return (
              <button
                key={s.id}
                type="button"
                role="option"
                aria-selected={on}
                onClick={() => {
                  setOpen(false);
                  if (!on) void set({ activeSource: s.id });
                }}
                className={`flex flex-col rounded-[16px] px-3.5 py-2 text-start ${on ? "bg-white text-navy" : "hover:bg-white/10"}`}
              >
                <span className="truncate text-[13px] font-extrabold">{s.name}</span>
                <span className={`truncate text-[10px] ${on ? "text-navy/60" : "text-ink-2"}`}>
                  {sub(s)} · {faDigits(s.links.length)} سرور
                </span>
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}

/** One server in Home's panel: flag, name, latency; white when it is the one. */
function PanelRow({ s, selected, onPick }: { s: ServerView; selected: boolean; onPick: () => void }) {
  return (
    <button
      type="button"
      aria-pressed={selected}
      onClick={onPick}
      className={`flex h-12 w-full shrink-0 items-center gap-3 rounded-2xl px-3 text-start transition-colors ${
        selected ? "bg-white text-navy" : "text-white hover:bg-white/8"
      }`}
    >
      <ServerMark name={s.name} />
      <span dir="auto" className="min-w-0 flex-1 truncate text-start text-sm font-bold">
        {serverName(s)}
      </span>
      <span dir="ltr" className={`shrink-0 font-num text-xs ${selected ? "text-navy/70" : "text-ink-2"}`}>
        {latency(s.delayMs)}
      </span>
    </button>
  );
}

/** Home's side panel (Desktop-Home): the subscription, search, auto select and its servers. */
export function ServerPanel() {
  const { view, state, testing, test, set } = useTunnel();
  const { servers } = useActiveSource();
  const [q, setQ] = useState("");
  const list = useMemo(() => servers.filter((s) => matches(s, q)), [servers, q]);
  const activeId = state.status === "on" ? state.serverId : state.status === "connecting" ? state.serverId : view?.selected;
  const auto = view?.autoSelect ?? true;

  return (
    <section className="glass-milk flex w-[320px] shrink-0 flex-col gap-3 rounded-[30px] p-4 max-[1250px]:w-[280px]">
      <div className="flex items-center justify-between gap-2 px-1">
        <span className="text-lg font-extrabold">سرورها</span>
        <button
          type="button"
          aria-label="تست تأخیر همه"
          title="تست تأخیر همه"
          disabled={testing || !servers.length}
          onClick={() => void test()}
          className="flex h-9 items-center gap-1.5 rounded-full px-3 text-xs font-bold text-white/85 hover:bg-white/10 disabled:opacity-50"
        >
          <span className={`flex ${testing ? "animate-pulse" : ""}`}>
            <Icon name="gauge" size={16} />
          </span>
          {testing ? "در حال تست" : "تست"}
        </button>
      </div>
      <SourcePicker />
      <SourceInfo />
      <SearchField id="server-search" placeholder="جستجوی سرور" value={q} onChange={setQ} />
      <button
        type="button"
        aria-pressed={auto}
        onClick={() => void set({ autoSelect: !auto })}
        className={`flex h-12 shrink-0 items-center gap-3 rounded-2xl px-3 text-start transition-colors ${auto ? "bg-white text-navy" : "glass-clear"}`}
      >
        <span className="flex w-7 justify-center">
          <Icon name="bolt" size={18} />
        </span>
        <span className="flex flex-1 flex-col leading-tight">
          <span className="text-sm font-extrabold">انتخاب خودکار</span>
          <span className={`text-[11px] ${auto ? "text-navy/60" : "text-ink-2"}`}>{auto ? "سریع‌ترین سرور همین اشتراک" : "خاموش · سرور دستی"}</span>
        </span>
      </button>
      <Hair />
      <div className="-mx-1 flex min-h-0 flex-1 flex-col gap-1 overflow-y-auto px-1 [scrollbar-width:thin]">
        {list.length === 0 ? (
          <span className="px-2 py-8 text-center text-[13px] leading-[1.9] text-ink-2">
            {servers.length ? "سروری با این جستجو نیست." : "با ورود به حسابت سرویس‌هایت می‌آیند؛ یا در «سرویس‌ها» لینک اضافه کن."}
          </span>
        ) : (
          list.map((s) => <PanelRow key={s.id} s={s} selected={!auto && s.id === activeId} onPick={() => void set({ selected: s.id })} />)
        )}
      </div>
    </section>
  );
}

const COLS = "grid grid-cols-[minmax(0,1.6fr)_minmax(0,1fr)_88px_64px_32px] items-center gap-3 max-[1250px]:grid-cols-[minmax(0,1.6fr)_64px_32px] max-[1250px]:[&>*:nth-child(2)]:hidden max-[1250px]:[&>*:nth-child(3)]:hidden";

function ServerLine({ s, active }: { s: ServerView; active: boolean }) {
  const { set } = useTunnel();
  return (
    <div className={`${COLS} min-h-12 rounded-2xl px-3 text-[13px] ${active ? "bg-white text-navy" : "hover:bg-white/5"}`}>
      <button type="button" onClick={() => void set({ selected: s.id })} aria-pressed={active} className="flex min-w-0 items-center gap-3 py-2.5 text-start">
        <ServerMark name={s.name} />
        <span dir="auto" className="min-w-0 truncate text-start text-sm font-extrabold">
          {serverName(s)}
        </span>
      </button>
      <span dir="ltr" className={`truncate text-right font-num text-xs ${active ? "text-navy/70" : "text-ink-2"}`}>
        {s.protocol} · {s.network}
        {s.security !== "none" ? ` · ${s.security}` : ""}
      </span>
      <span className="flex">
        {s.cdn && (
          <span className={`inline-flex h-6 items-center whitespace-nowrap rounded-full border px-2.5 text-[11px] font-bold ${active ? "border-navy/25" : "border-white/25 bg-white/8"}`}>پشت CDN</span>
        )}
      </span>
      <span dir="ltr" className="text-right font-num text-[13px]">
        {latency(s.delayMs)}
      </span>
      <button
        type="button"
        aria-label={s.favorite ? "برداشتن ستاره" : "ستاره زدن"}
        aria-pressed={s.favorite}
        onClick={() => void set({ favorite: [s.id, !s.favorite] })}
        className="flex h-8 w-8 items-center justify-center rounded-full hover:bg-white/10"
      >
        <Icon name="star" size={16} color={s.favorite ? (active ? "#07123A" : "#FFFFFF") : active ? "rgba(7,18,58,0.35)" : "rgba(255,255,255,0.35)"} />
      </button>
    </div>
  );
}

/** Desktop-Servers: one subscription's servers, starred first. */
export function ServersPage() {
  const { view, state, testing, test, set, error } = useTunnel();
  const { active, servers } = useActiveSource();
  const [q, setQ] = useState("");
  const list = useMemo(() => servers.filter((s) => matches(s, q)), [servers, q]);
  const activeId = state.status === "on" ? state.serverId : view?.selected;
  const auto = view?.autoSelect ?? true;
  const starred = list.filter((s) => s.favorite);
  const rest = list.filter((s) => !s.favorite);

  return (
    <>
      <div className="flex min-w-0 flex-1 flex-col gap-[18px]">
        <PageHeader
          micro="Servers"
          title="سرورها"
          subtitle={active ? `${faDigits(servers.length)} سرور در «${active.name}»` : undefined}
          actions={
            <>
              <div className="w-[180px]">
                <SearchField id="sq" placeholder="جستجوی سرور" value={q} onChange={setQ} />
              </div>
              <Button kind="soft" icon="sort" height={40} aria-pressed={view?.sortByPing} onClick={() => void set({ sortByPing: !view?.sortByPing })}>
                {view?.sortByPing ? "کمترین پینگ" : "ترتیب اشتراک"}
              </Button>
              <Button icon="gauge" height={40} disabled={testing || !servers.length} onClick={() => void test()}>
                {testing ? "در حال تست…" : "تست تأخیر همه"}
              </Button>
            </>
          }
        />
        <div className="flex flex-wrap items-center gap-3">
          <SourcePicker wide />
          <SourceInfo />
        </div>
        {error && <span role="alert" className="text-[13px] text-bad">{error}</span>}
        <div className="flex min-h-0 flex-1 flex-col overflow-y-auto pe-1 [scrollbar-width:thin]">
          {list.length === 0 ? (
            <div className="p-10 text-center text-[13px] text-ink-2">
              {servers.length ? "سروری با این جستجو نیست." : "هنوز سروری نداری. وارد حسابت شو یا در «سرویس‌ها» لینک اضافه کن."}
            </div>
          ) : (
            <div className="flex flex-col gap-1">
              <div className={`${COLS} px-3 pb-1`}>
                {["Server", "Protocol", "", "Latency", ""].map((h, i) => (
                  <span key={i} className="micro text-[9px] tracking-[0.2em]">
                    {h}
                  </span>
                ))}
              </div>
              <Hair />
              {starred.length > 0 && (
                <>
                  <span className="micro px-3 pt-2 text-[9px]">Starred</span>
                  {starred.map((s) => (
                    <ServerLine key={s.id} s={s} active={!auto && s.id === activeId} />
                  ))}
                  {rest.length > 0 && <span className="micro px-3 pt-3 text-[9px]">All</span>}
                </>
              )}
              {rest.map((s) => (
                <ServerLine key={s.id} s={s} active={!auto && s.id === activeId} />
              ))}
            </div>
          )}
        </div>
      </div>
      <div className="flex w-[330px] shrink-0 flex-col gap-4 max-[1250px]:w-[270px]">
        <AutoCard />
        <OptimizerCard />
      </div>
    </>
  );
}

const FAILOVER = [
  { value: "off", label: "هرگز" },
  { value: "lost", label: "قطعی" },
  { value: "1000", label: "۱ ث" },
  { value: "2000", label: "۲ ث" },
  { value: "3000", label: "۳ ث" },
] as const satisfies readonly { value: Failover; label: string }[];

/** «انتخاب خودکار»: smart connect, and failover while connected. */
function AutoCard() {
  const { view, set } = useTunnel();
  const auto = view?.autoSelect ?? true;
  return (
    <Card>
      <CardTitle micro="Auto select" title="اتصال هوشمند و failover" />
      <div className="flex items-center justify-between">
        <span className="text-[13px]">سرور: خودکار</span>
        <Switch label="سرور خودکار" checked={auto} onChange={(v) => void set({ autoSelect: v })} />
      </div>
      <Hair />
      <span className="text-xs text-ink-2">جابه‌جایی وقتی تأخیر بیشتر شد از</span>
      <div className={auto ? "" : "pointer-events-none opacity-55"}>
        <Segmented label="آستانه‌ی failover" options={FAILOVER} value={view?.failover ?? "2000"} onChange={(f) => void set({ failover: f })} height={36} />
      </div>
      <span className="text-[11px] leading-[1.7] text-ink-2">
        {auto ? "دو اندازه‌گیری بد پشت سر هم = رفتن به سرور بهتر، بدون قطع تونل." : "با انتخاب دستی سرور، failover خاموش است."}
      </span>
    </Card>
  );
}

/** The clean-IP scanner at a glance, for the selected server. */
function OptimizerCard() {
  const { view } = useTunnel();
  const [scan, setScan] = useState<ScanView | null>(null);
  useEffect(() => {
    if (inTauri) void scanner.state().then(setScan, () => {});
  }, [view?.selected]);
  if (!scan || scan.candidates.length === 0) return null;
  const best = scan.results[0];
  return (
    <Card className="flex-1">
      <CardTitle micro="Cloudflare optimizer" title="بهینه‌ساز کلادفلر" subtitle="برای سرویس‌های مستقیم پشت CDN" />
      <Hair />
      <div className="flex gap-3.5">
        <div className="flex flex-1 flex-col gap-0.5">
          <span className="micro text-[9px]">Clean IPs</span>
          <span className="text-[28px] font-extrabold">{faDigits(scan.results.length)}</span>
        </div>
        <Hair vertical />
        <div className="flex flex-1 flex-col gap-0.5">
          <span className="micro text-[9px]">Best</span>
          <span dir="ltr" className="text-right font-num text-[26px] font-medium">
            {best ? `${best.latencyMs} ms` : "—"}
          </span>
        </div>
      </div>
      <span className="text-[11px] text-ink-2">شبکه: {scan.network.label}</span>
      <Link to="/tools" className="glass-milk flex h-10 items-center justify-center gap-2 self-start rounded-full px-4 text-[13px] font-bold text-white no-underline">
        <Icon name="radar" size={16} />
        {scan.results.length ? "اسکن دوباره" : "اسکن IP تمیز"}
      </Link>
    </Card>
  );
}
