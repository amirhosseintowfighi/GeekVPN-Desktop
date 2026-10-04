import { useEffect, useMemo, useState } from "react";
import { Link } from "react-router";
import { Icon } from "../design-system/Icon";
import { CountryBadge, PingBars, SearchField, Segmented, Switch } from "../design-system/controls";
import { Card, CardTitle } from "../design-system/layout";
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

/** Home's right-hand panel (Desktop-Home): compact list, auto switch, test and refresh. */
export function ServerPanel() {
  const { view, state, testing, test, refresh, set } = useTunnel();
  const [filter, setFilter] = useState("all");
  const [q, setQ] = useState("");
  const filters = useFilters();
  const list = useMemo(() => (view?.servers ?? []).filter((s) => matches(s, filter, q)), [view, filter, q]);
  const activeId = state.status === "on" ? state.serverId : state.status === "connecting" ? state.serverId : view?.selected;

  return (
    <section className="glass-milk flex w-[360px] shrink-0 flex-col gap-3 rounded-[26px] p-4">
      <div className="flex items-center gap-2">
        <span className="flex flex-1 flex-col">
          <span className="text-[17px] font-extrabold">سرورها</span>
          <span className="text-xs text-ink-2">
            {view?.servers.length ? `${view.servers.length} سرور · ستاره‌دارها بالا` : "هنوز سروری اضافه نشده"}
          </span>
        </span>
        <button
          type="button"
          aria-label="تست تأخیر همه"
          title="تست تأخیر همه"
          disabled={testing || !view?.servers.length}
          onClick={() => void test()}
          className="flex h-[38px] w-[38px] items-center justify-center rounded-[13px] bg-soft text-link disabled:opacity-50"
        >
          <span className={testing ? "animate-pulse" : ""}>
            <Icon name="gauge" size={19} />
          </span>
        </button>
        <button
          type="button"
          aria-label="بروزرسانی سرویس‌ها"
          title="بروزرسانی سرویس‌ها"
          onClick={() => void refresh()}
          className="flex h-[38px] w-[38px] items-center justify-center rounded-[13px] bg-soft text-link"
        >
          <Icon name="refresh" size={19} />
        </button>
      </div>
      <SearchField id="server-search" placeholder="جستجوی سرور" value={q} onChange={setQ} />
      <div className="flex flex-wrap gap-1.5">
        {filters.slice(0, 4).map((f) => (
          <button
            key={f.id}
            type="button"
            aria-pressed={filter === f.id}
            onClick={() => setFilter(f.id)}
            className={`h-8 max-w-[140px] truncate rounded-[10px] px-3 text-xs font-bold ${
              filter === f.id ? "bg-action text-on-action" : "border border-chip-border bg-chip text-ink"
            }`}
          >
            {f.label}
          </button>
        ))}
      </div>
      <div className="flex items-center gap-2 rounded-2xl bg-soft-button p-2.5">
        <span className="flex flex-1 flex-col">
          <span className="text-xs font-extrabold">انتخاب خودکار</span>
          <span className="text-[10px] text-ink-2">سریع‌ترین سرور با هر اتصال</span>
        </span>
        <Switch checked={view?.autoSelect ?? true} onChange={(on) => void set({ autoSelect: on })} label="انتخاب خودکار" />
      </div>
      <div className="-mx-1 flex min-h-0 flex-1 flex-col gap-0.5 overflow-y-auto px-1">
        {list.length === 0 ? (
          <span className="py-8 text-center text-[13px] leading-[1.9] text-ink-2">
            {view?.servers.length ? "سروری با این جستجو نیست." : "با ورود به حسابت سرویس‌هایت می‌آیند؛ یا در «سرویس‌ها» لینک اضافه کن."}
          </span>
        ) : (
          list.map((s) => <ServerRow key={s.id} s={s} active={s.id === activeId} />)
        )}
      </div>
    </section>
  );
}

function ServerRow({ s, active }: { s: ServerView; active: boolean }) {
  const { set } = useTunnel();
  return (
    <div
      className={`flex items-center gap-2.5 rounded-[14px] border-2 px-2 py-1.5 ${active ? "border-link bg-soft" : "border-transparent"}`}
    >
      <button
        type="button"
        aria-pressed={active}
        onClick={() => void set({ selected: s.id })}
        className="flex min-w-0 flex-1 items-center gap-2.5 text-start"
      >
        <CountryBadge code={countryCode(s.name)} selected={active} size={36} />
        <span className="flex min-w-0 flex-1 flex-col">
          <span className="truncate text-[13px] font-extrabold">{plainName(s.name) || s.address}</span>
          <span dir="ltr" className="truncate text-end font-num text-[11px] text-ink-2">
            {s.protocol} · {s.network}
            {s.security !== "none" ? ` · ${s.security}` : ""}
          </span>
        </span>
        <PingBars ms={s.delayMs ?? 0} />
      </button>
      <button
        type="button"
        aria-label={s.favorite ? "برداشتن ستاره" : "ستاره زدن"}
        aria-pressed={s.favorite}
        onClick={() => void set({ favorite: [s.id, !s.favorite] })}
        className="flex h-8 w-8 items-center justify-center"
      >
        <Icon name="star" size={17} color={s.favorite ? "var(--gv-warn)" : "var(--gv-check)"} />
      </button>
    </div>
  );
}

/** Desktop-Servers: the full table with protocol, address and sorting. */
export function ServersPage() {
  const { view, state, testing, test, set, error } = useTunnel();
  const [filter, setFilter] = useState("all");
  const [q, setQ] = useState("");
  const filters = useFilters();
  const list = useMemo(() => (view?.servers ?? []).filter((s) => matches(s, filter, q)), [view, filter, q]);
  const activeId = state.status === "on" ? state.serverId : view?.selected;

  return (
    <div className="flex min-w-0 flex-1 flex-col gap-3.5">
      <header className="flex items-center gap-3">
        <div className="flex flex-1 flex-col gap-0.5">
          <h1 className="m-0 text-[26px] font-extrabold">سرورها</h1>
          <span className="text-[13px] opacity-90">
            {view ? `${view.servers.length} سرور از ${view.sources.filter((s) => s.links.length).length} منبع` : ""}
          </span>
        </div>
        <label htmlFor="sq" className="sr-only">
          جستجوی سرور
        </label>
        <div className="glass-clear flex h-[42px] w-[240px] items-center gap-2 rounded-[13px] px-3">
          <Icon name="search" size={17} />
          <input
            id="sq"
            value={q}
            onChange={(e) => setQ(e.target.value)}
            placeholder="جستجوی سرور"
            className="min-w-0 flex-1 border-none bg-transparent text-[13px] text-white outline-none placeholder:text-white/70"
          />
        </div>
        <button
          type="button"
          aria-pressed={view?.sortByPing}
          onClick={() => void set({ sortByPing: !view?.sortByPing })}
          className="glass-clear flex h-[42px] items-center gap-2 rounded-[13px] px-3.5 text-[13px] font-bold"
        >
          <Icon name="sort" size={17} />
          {view?.sortByPing ? "کمترین پینگ" : "ترتیب اشتراک"}
        </button>
        <button
          type="button"
          disabled={testing || !view?.servers.length}
          onClick={() => void test()}
          className="flex h-[42px] items-center gap-2.5 rounded-[13px] bg-white pe-1.5 ps-3.5 text-[13px] font-bold text-[#062845] disabled:opacity-60"
        >
          {testing ? "در حال تست…" : "تست تأخیر همه"}
          <span className="flex h-[30px] w-[30px] items-center justify-center rounded-[10px] bg-[#062845] text-logo">
            <Icon name="gauge" size={16} />
          </span>
        </button>
      </header>
      <div className="flex flex-wrap gap-1.5">
        {filters.map((f) => (
          <button
            key={f.id}
            type="button"
            aria-pressed={filter === f.id}
            onClick={() => setFilter(f.id)}
            className={`h-9 rounded-xl px-3.5 text-[13px] font-bold ${filter === f.id ? "bg-white text-[#062845]" : "glass-clear"}`}
          >
            {f.label}
          </button>
        ))}
      </div>
      {error && <span className="text-[13px]">{error}</span>}
      <div className="flex min-h-0 flex-1 gap-4 max-[1200px]:flex-col">
      <section className="glass-milk flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden rounded-[26px]">
        <div className="grid grid-cols-[44px_minmax(0,1.6fr)_minmax(0,1.3fr)_110px_120px_44px] gap-2.5 border-b border-hair px-4 py-3 text-xs font-bold text-ink-2">
          <span />
          <span>سرور</span>
          <span>پروتکل</span>
          <span>آدرس</span>
          <span>تأخیر واقعی</span>
          <span />
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto">
          {list.length === 0 && (
            <div className="p-10 text-center text-[13px] text-ink-2">
              {view?.servers.length ? "سروری با این فیلتر نیست." : "هنوز سروری نداری. وارد حسابت شو یا در «سرویس‌ها» لینک اضافه کن."}
            </div>
          )}
          {list.map((s) => {
            const active = s.id === activeId;
            return (
              <div
                key={s.id}
                className={`grid grid-cols-[44px_minmax(0,1.6fr)_minmax(0,1.3fr)_110px_120px_44px] items-center gap-2.5 px-4 py-2 ${active ? "bg-soft" : ""}`}
              >
                <CountryBadge code={countryCode(s.name)} selected={active} />
                <button type="button" onClick={() => void set({ selected: s.id })} className="flex min-w-0 flex-col text-start">
                  <span className="truncate text-sm font-extrabold">{plainName(s.name) || s.address}</span>
                  <span className="truncate text-[11px] text-ink-2">{view?.sources.find((x) => x.id === s.sourceId)?.name}</span>
                </button>
                <span dir="ltr" className="truncate text-end font-num text-xs text-ink-2">
                  {s.protocol} · {s.network} · {s.security}
                </span>
                <span className="flex">
                  {s.cdn && (
                    <span className="rounded-[9px] bg-soft px-2.5 py-1 text-xs font-extrabold text-link">پشت CDN</span>
                  )}
                </span>
                <PingBars ms={s.delayMs ?? 0} />
                <button
                  type="button"
                  aria-label={s.favorite ? "برداشتن ستاره" : "ستاره زدن"}
                  aria-pressed={s.favorite}
                  onClick={() => void set({ favorite: [s.id, !s.favorite] })}
                  className="flex h-9 w-9 items-center justify-center"
                >
                  <Icon name="star" size={19} color={s.favorite ? "var(--gv-warn)" : "var(--gv-check)"} />
                </button>
              </div>
            );
          })}
        </div>
      </section>
      <div className="flex w-[300px] shrink-0 flex-col gap-4 max-[1200px]:w-full max-[1200px]:flex-row max-[1200px]:flex-wrap [&>*]:max-[1200px]:flex-1">
        <AutoCard />
        <OptimizerCard />
      </div>
      </div>
    </div>
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
      <CardTitle title="انتخاب خودکار" subtitle="اتصال هوشمند و failover" />
      <div className="flex items-center gap-2.5 rounded-[14px] bg-soft-button px-3 py-2.5">
        <span className="flex-1 text-[13px] font-bold">سرور: خودکار</span>
        <Switch label="سرور خودکار" checked={auto} onChange={(v) => void set({ autoSelect: v })} />
      </div>
      <span className="text-xs font-bold text-ink-2">جابه‌جایی وقتی تأخیر بیشتر شد از</span>
      <div className={auto ? "" : "pointer-events-none opacity-55"}>
        <Segmented label="آستانه‌ی failover" options={FAILOVER} value={view?.failover ?? "2000"} onChange={(f) => void set({ failover: f })} height={38} />
      </div>
      <span className="text-xs leading-[1.8] text-ink-2">
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
    <Card>
      <CardTitle title="بهینه‌ساز کلادفلر" subtitle="برای سرویس‌های مستقیم پشت CDN" />
      <div className="flex gap-2.5">
        <div className="flex flex-1 flex-col gap-0.5 rounded-[14px] bg-soft-button p-2.5">
          <span className="text-[11px] text-ink-2">IP تمیز این شبکه</span>
          <span className="font-num text-xl font-bold">{faDigits(scan.results.length)}</span>
        </div>
        <div className="flex flex-1 flex-col gap-0.5 rounded-[14px] bg-soft-button p-2.5">
          <span className="text-[11px] text-ink-2">بهترین</span>
          <span dir="ltr" className="text-right font-num text-xl font-bold">
            {best ? `${best.latencyMs}ms` : "—"}
          </span>
        </div>
      </div>
      <span className="text-xs text-ink-2">شبکه: {scan.network.label}</span>
      <Link to="/tools" className="flex h-11 items-center justify-center gap-2 rounded-[14px] bg-action text-sm font-bold text-on-action no-underline">
        <Icon name="radar" size={18} />
        {scan.results.length ? "اسکن دوباره" : "اسکن IP تمیز"}
      </Link>
    </Card>
  );
}
