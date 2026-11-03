import { useEffect, useMemo, useState } from "react";
import { Link } from "react-router";
import { Icon } from "../design-system/Icon";
import { Button, Hair, RuleRow, SearchField, Segmented, Switch } from "../design-system/controls";
import { Card, CardTitle, PageHeader } from "../design-system/layout";
import { faDigits } from "../lib/fa";
import { countryCode, plainName } from "../lib/format";
import { inTauri } from "../lib/platform";
import { scanner, type Failover, type ScanView, type ServerView } from "../lib/servers";
import { useTunnel } from "../lib/TunnelContext";

/** Filter chips: all, starred, then one per source. */
function useFilters() {
  const { view } = useTunnel();
  const sources = view?.sources.filter((s) => s.links.length > 0) ?? [];
  return [
    { id: "all", label: "همه" },
    { id: "fav", label: "ستاره‌دار" },
    ...sources.map((s) => ({ id: s.id, label: s.name })),
  ];
}

function matches(s: ServerView, filter: string, q: string): boolean {
  if (filter === "fav" && !s.favorite) return false;
  if (filter !== "all" && filter !== "fav" && s.sourceId !== filter) return false;
  const needle = q.trim().toLowerCase();
  return !needle || s.name.toLowerCase().includes(needle) || s.address.toLowerCase().includes(needle);
}

function latency(ms: number | null | undefined): string {
  return ms === null || ms === undefined ? "—" : ms > 0 ? `${ms} ms` : "×";
}

/** Home's side panel (Desktop-Home): search, sources, auto select and the list. */
export function ServerPanel() {
  const { view, state, testing, test, set } = useTunnel();
  const [filter, setFilter] = useState("all");
  const [q, setQ] = useState("");
  const filters = useFilters().filter((f) => f.id !== "fav").slice(0, 3);
  const list = useMemo(() => (view?.servers ?? []).filter((s) => matches(s, filter, q)), [view, filter, q]);
  const activeId = state.status === "on" ? state.serverId : state.status === "connecting" ? state.serverId : view?.selected;
  const auto = view?.autoSelect ?? true;

  return (
    <section className="glass-milk flex w-[320px] shrink-0 flex-col gap-3 rounded-[30px] p-5">
      <div className="flex items-center justify-between gap-2">
        <span className="text-lg font-extrabold">سرورها</span>
        <span className="flex items-center gap-2">
          <span className="micro text-[9px]">{view?.servers.length ?? 0} available</span>
          <button
            type="button"
            aria-label="تست تأخیر همه"
            title="تست تأخیر همه"
            disabled={testing || !view?.servers.length}
            onClick={() => void test()}
            className="flex h-8 w-8 items-center justify-center rounded-full text-white/80 hover:bg-white/10 disabled:opacity-50"
          >
            <span className={testing ? "animate-pulse" : ""}>
              <Icon name="gauge" size={17} />
            </span>
          </button>
        </span>
      </div>
      <SearchField id="server-search" placeholder="جستجوی سرور" value={q} onChange={setQ} />
      {filters.length > 1 && <Segmented label="منبع" options={filters.map((f) => ({ value: f.id, label: f.label }))} value={filter} onChange={setFilter} height={40} />}
      <button
        type="button"
        aria-pressed={auto}
        onClick={() => void set({ autoSelect: !auto })}
        className={`flex items-center gap-2.5 rounded-[18px] px-3.5 py-2.5 text-start transition-colors ${auto ? "bg-white text-navy" : "glass-clear"}`}
      >
        <Icon name="bolt" size={18} />
        <span className="flex flex-1 flex-col">
          <span className="text-sm font-extrabold">انتخاب خودکار</span>
          <span className="text-[11px] opacity-70">{auto ? "سریع‌ترین سرور" : "خاموش · سرور دستی"}</span>
        </span>
      </button>
      <div className="-mx-1 flex min-h-0 flex-1 flex-col overflow-y-auto px-1 [scrollbar-width:thin]">
        {list.length === 0 ? (
          <span className="py-8 text-center text-[13px] leading-[1.9] text-ink-2">
            {view?.servers.length ? "سروری با این جستجو نیست." : "با ورود به حسابت سرویس‌هایت می‌آیند؛ یا در «سرویس‌ها» لینک اضافه کن."}
          </span>
        ) : (
          <>
            <Hair />
            {list.map((s) => (
              <RuleRow
                key={s.id}
                code={countryCode(s.name)}
                title={plainName(s.name) || s.address}
                hint={view?.sources.find((x) => x.id === s.sourceId)?.name}
                end={<span dir="ltr" className="font-num">{latency(s.delayMs)}</span>}
                size={14}
                selected={!auto && s.id === activeId}
                onClick={() => void set({ selected: s.id })}
              />
            ))}
          </>
        )}
      </div>
    </section>
  );
}

const COLS = "grid grid-cols-[150px_minmax(0,1fr)_150px_80px_70px_28px] items-center gap-3.5";

function ServerLine({ s, active, source }: { s: ServerView; active: boolean; source: string }) {
  const { set } = useTunnel();
  return (
    <div className={`${COLS} border-b border-hair py-[5px] text-[13px] ${active ? "-mx-3 rounded-[18px] border-transparent bg-white px-3 text-navy" : ""}`}>
      <button type="button" onClick={() => void set({ selected: s.id })} aria-pressed={active} className="flex min-w-0 items-center gap-2.5 py-1.5 text-start">
        <span dir="ltr" className={`w-6 shrink-0 font-num text-[11px] tracking-[0.18em] ${active ? "" : "text-ink-2"}`}>
          {countryCode(s.name)}
        </span>
        <span className="truncate text-sm font-extrabold">{plainName(s.name) || s.address}</span>
      </button>
      <span className={`truncate text-xs ${active ? "" : "text-ink-2"}`}>{source}</span>
      <span dir="ltr" className={`truncate text-right font-num text-xs ${active ? "" : "text-ink-2"}`}>
        {s.protocol} · {s.network}
        {s.security !== "none" ? ` · ${s.security}` : ""}
      </span>
      <span className="flex">
        {s.cdn && (
          <span className={`inline-flex h-6 items-center whitespace-nowrap rounded-full border px-2.5 text-[11px] font-extrabold ${active ? "border-navy/30" : "border-white/25 bg-white/12"}`}>پشت CDN</span>
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
        className="flex h-7 w-7 items-center justify-center"
      >
        <Icon name="star" size={16} color={s.favorite ? (active ? "#07123A" : "#FFFFFF") : active ? "rgba(7,18,58,0.35)" : "rgba(255,255,255,0.35)"} />
      </button>
    </div>
  );
}

/** Desktop-Servers: the ruled table, starred first, then one block per source. */
export function ServersPage() {
  const { view, state, testing, test, set, error } = useTunnel();
  const [filter, setFilter] = useState("all");
  const [q, setQ] = useState("");
  const filters = useFilters();
  const list = useMemo(() => (view?.servers ?? []).filter((s) => matches(s, filter, q)), [view, filter, q]);
  const activeId = state.status === "on" ? state.serverId : view?.selected;
  const sourceName = (id: string) => view?.sources.find((x) => x.id === id)?.name ?? "";
  const count = (id: string) => (view?.servers ?? []).filter((s) => matches(s, id, "")).length;
  const starred = filter === "all" ? list.filter((s) => s.favorite) : [];
  const groups = (view?.sources ?? [])
    .map((src) => ({ src, servers: list.filter((s) => s.sourceId === src.id && !(filter === "all" && s.favorite)) }))
    .filter((g) => g.servers.length > 0);

  return (
    <>
      <div className="flex min-w-0 flex-1 flex-col gap-[18px]">
        <PageHeader
          micro="Servers"
          title="سرورها"
          subtitle={view ? `${faDigits(view.servers.length)} سرور از ${faDigits(view.sources.filter((s) => s.links.length).length)} منبع` : undefined}
          actions={
            <>
              <div className="w-[180px]">
                <SearchField id="sq" placeholder="جستجوی سرور" value={q} onChange={setQ} />
              </div>
              <Button kind="soft" icon="sort" height={40} aria-pressed={view?.sortByPing} onClick={() => void set({ sortByPing: !view?.sortByPing })}>
                {view?.sortByPing ? "کمترین پینگ" : "ترتیب اشتراک"}
              </Button>
              <Button icon="gauge" height={40} disabled={testing || !view?.servers.length} onClick={() => void test()}>
                {testing ? "در حال تست…" : "تست تأخیر همه"}
              </Button>
            </>
          }
        />
        {filters.length > 1 && (
          <Segmented label="فیلتر سرورها" height={42} options={filters.slice(0, 5).map((f) => ({ value: f.id, label: `${f.label} · ${faDigits(count(f.id))}` }))} value={filter} onChange={setFilter} />
        )}
        {error && <span role="alert" className="text-[13px] text-bad">{error}</span>}
        <div className="flex min-h-0 flex-1 flex-col overflow-y-auto pe-1 [scrollbar-width:thin]">
          {list.length === 0 ? (
            <div className="p-10 text-center text-[13px] text-ink-2">
              {view?.servers.length ? "سروری با این فیلتر نیست." : "هنوز سروری نداری. وارد حسابت شو یا در «سرویس‌ها» لینک اضافه کن."}
            </div>
          ) : (
            <>
              <div className={`${COLS} pb-2.5`}>
                {["Server", "Source", "Protocol", "", "Latency", ""].map((h, i) => (
                  <span key={i} className="micro text-[9px] tracking-[0.2em]">{h}</span>
                ))}
              </div>
              {starred.length > 0 && (
                <>
                  <span className="micro pb-0.5 pt-2 text-[9px]">Starred</span>
                  <Hair />
                  {starred.map((s) => <ServerLine key={s.id} s={s} active={s.id === activeId} source={sourceName(s.sourceId)} />)}
                </>
              )}
              {groups.map(({ src, servers }) => (
                <div key={src.id} className="flex flex-col">
                  <span className="pb-0.5 pt-3 text-xs font-bold text-ink-2">{src.name}</span>
                  <Hair />
                  {servers.map((s) => <ServerLine key={s.id} s={s} active={s.id === activeId} source={src.name} />)}
                </div>
              ))}
            </>
          )}
        </div>
      </div>
      <div className="flex w-[330px] shrink-0 flex-col gap-4">
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
