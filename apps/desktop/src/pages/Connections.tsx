import { useEffect, useMemo, useState } from "react";
import { useNavigate } from "react-router";
import { Badge, Button, ClearChips, IconButton, SearchField } from "../design-system/controls";
import { EmptyState, PageHeader } from "../design-system/layout";
import { errorText } from "../lib/auth";
import { faDigits } from "../lib/fa";
import { ago, bytes, countryCode, plainName } from "../lib/format";
import { system, type Connection } from "../lib/servers";
import { useTunnel } from "../lib/TunnelContext";

type Filter = "all" | "proxy" | "direct";

/** Refreshed once a second while the page is open, like the Android list. */
const POLL_MS = 1000;

export function Connections() {
  const { state, view } = useTunnel();
  const navigate = useNavigate();
  const [list, setList] = useState<Connection[] | null>(null);
  const [available, setAvailable] = useState(false);
  const [filter, setFilter] = useState<Filter>("all");
  const [query, setQuery] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [now, setNow] = useState(Date.now());

  const on = state.status === "on";
  const tun = on && state.mode === "tun";

  useEffect(() => {
    if (!tun) {
      setList(null);
      setAvailable(false);
      return;
    }
    let live = true;
    const load = () =>
      system.connections().then(
        (v) => {
          if (!live) return;
          setAvailable(v.available);
          setList(v.connections);
          setNow(Date.now());
          setError(null);
        },
        (e) => live && setError(errorText(e)),
      );
    void load();
    const t = setInterval(load, POLL_MS);
    return () => {
      live = false;
      clearInterval(t);
    };
  }, [tun]);

  const counts = useMemo(() => {
    const all = list ?? [];
    return { all: all.length, proxy: all.filter((c) => c.route === "proxy").length, direct: all.filter((c) => c.route === "direct").length };
  }, [list]);

  const shown = useMemo(() => {
    const q = query.trim().toLowerCase();
    return (list ?? []).filter(
      (c) => (filter === "all" || c.route === filter) && (!q || c.host.toLowerCase().includes(q) || c.process.toLowerCase().includes(q)),
    );
  }, [list, filter, query]);

  const close = (id?: string) => void system.closeConnection(id).catch((e) => setError(errorText(e)));

  const server = on ? view?.servers.find((s) => s.id === state.serverId) : undefined;
  const through = server ? `VPN · ${countryCode(server.name)}` : "VPN";

  if (!tun || !available) {
    return (
      <div className="flex min-w-0 flex-1 flex-col gap-3.5">
        <PageHeader title="اتصالات" />
        <EmptyState
          icon="hub"
          title={on ? "فهرست اتصال‌ها در حالت TUN" : "اتصالی باز نیست"}
          text={
            on
              ? "در حالت پروکسی سیستم، هر برنامه اتصالش را خودش می‌سازد و برنامه آن‌ها را نمی‌بیند. در حالت TUN، هر اتصال با مقصد، برنامه، حجم و مسیرش اینجا دیده می‌شود."
              : "وقتی در حالت TUN وصل باشی، هر اتصال با مقصد، برنامه، حجم و مسیرش اینجا دیده می‌شود."
          }
          action={
            on ? (
              <Button kind="white" icon="gear" height={40} onClick={() => navigate("/settings")}>
                تنظیمات حالت اتصال
              </Button>
            ) : undefined
          }
        />
      </div>
    );
  }

  return (
    <div className="flex min-w-0 flex-1 flex-col gap-3.5">
      <PageHeader
        title="اتصالات"
        subtitle={`${faDigits(counts.all)} اتصال فعال${server ? ` از طریق ${plainName(server.name) || server.address}` : ""}`}
        actions={
          <Button kind="clear" icon="x" height={40} disabled={!counts.all} onClick={() => close()}>
            بستن همه
          </Button>
        }
      />
      <div className="flex items-center gap-3">
        <ClearChips
          options={[
            { value: "all", label: `همه ${faDigits(counts.all)}` },
            { value: "proxy", label: `VPN ${faDigits(counts.proxy)}` },
            { value: "direct", label: `مستقیم ${faDigits(counts.direct)}` },
          ]}
          value={filter}
          onChange={setFilter}
        />
        <span className="flex-1" />
        <div className="w-[280px]">
          <SearchField id="conn-search" placeholder="جستجوی مقصد یا برنامه" value={query} onChange={setQuery} />
        </div>
      </div>
      {error && (
        <span role="alert" className="rounded-xl bg-white/15 px-3 py-2 text-xs">
          {error}
        </span>
      )}
      <section className="glass-milk flex min-h-0 flex-1 flex-col overflow-hidden rounded-[26px]">
        <div className="grid grid-cols-[minmax(0,1fr)_90px_90px_110px_100px_36px] gap-3 border-b border-hair px-4 py-2.5 text-xs font-bold text-ink-2">
          <span>مقصد و برنامه</span>
          <span>آپلود</span>
          <span>دانلود</span>
          <span>مسیر</span>
          <span>زمان</span>
          <span />
        </div>
        <div className="flex-1 overflow-y-auto">
          {shown.map((c) => (
            <div
              key={c.id}
              className="grid grid-cols-[minmax(0,1fr)_90px_90px_110px_100px_36px] items-center gap-3 border-b border-hair px-4 py-2 last:border-b-0"
            >
              <span className="flex min-w-0 items-center gap-2.5">
                <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-soft font-num text-sm font-bold text-link">
                  {(c.process || c.host).slice(0, 1).toUpperCase()}
                </span>
                <span className="flex min-w-0 flex-col">
                  <span dir="ltr" className="truncate text-right font-num text-[13px] font-bold text-ink">
                    {c.host}:{c.port}
                    {c.network === "udp" ? " · UDP" : ""}
                  </span>
                  <span dir="ltr" className="truncate text-right font-num text-[11px] text-muted" title={c.processPath}>
                    {c.process || "—"}
                  </span>
                </span>
              </span>
              <span dir="ltr" className="text-right font-num text-xs text-ink">
                ↑ {bytes(c.upload)}
              </span>
              <span dir="ltr" className="text-right font-num text-xs text-ink">
                ↓ {bytes(c.download)}
              </span>
              <span>{c.route === "proxy" ? <Badge tone="link">{through}</Badge> : <Badge tone="ok">مستقیم</Badge>}</span>
              <span className="text-xs text-ink-2">{faDigits(ago(c.start, now))}</span>
              <IconButton icon="x" label={`بستن اتصال ${c.host}`} size={32} onClick={() => close(c.id)} />
            </div>
          ))}
          {shown.length === 0 && <p className="m-0 p-6 text-center text-[13px] text-ink-2">{list === null ? "در حال خواندن…" : "اتصالی با این فیلتر نیست."}</p>}
        </div>
      </section>
    </div>
  );
}
