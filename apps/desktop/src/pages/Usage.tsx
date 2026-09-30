import { useEffect, useMemo, useState } from "react";
import { useSearchParams } from "react-router";
import { Segmented } from "../design-system/controls";
import { Card, CardTitle, PageHeader } from "../design-system/layout";
import { sizeFa, usage, usageSummary, type DayUsage } from "../lib/account";
import { errorText } from "../lib/auth";
import { faDigits } from "../lib/fa";
import { useTunnel } from "../lib/TunnelContext";

const RANGES = [
  { value: "7", label: "۷ روز" },
  { value: "30", label: "۳۰ روز" },
] as const;

function dayLabel(day: string): string {
  return new Date(`${day}T12:00:00`).toLocaleDateString("fa-IR", { month: "long", day: "numeric" });
}

function Stat({ label, bytes, sub }: { label: string; bytes?: number; sub?: string }) {
  const s = bytes === undefined ? null : sizeFa(bytes);
  return (
    <Card className="flex-1 basis-0">
      <span className="text-xs text-ink-2">{label}</span>
      <span className="text-[28px] font-extrabold leading-tight text-ink">
        {s ? s.value : sub} {s && <span className="text-[13px] text-ink-2">{s.unit}</span>}
      </span>
      {s && sub && <span className="text-[11px] text-ink-2">{sub}</span>}
    </Card>
  );
}

function Chart({ days }: { days: DayUsage[] }) {
  const W = 760;
  const H = 260;
  const pad = 36;
  const max = Math.max(...days.map((d) => d.bytes), 1);
  // A round top in the unit the biggest day is shown in.
  const unit = max >= 1024 ** 3 ? 1024 ** 3 : 1024 ** 2;
  const top = Math.max(1, Math.ceil(max / unit));
  const bw = (W - pad) / Math.max(days.length, 1);
  const y = (v: number) => H - 24 - ((H - 40) * v) / (top * unit);
  const grid = [0, 0.25, 0.5, 0.75, 1].map((g) => g * top);
  const every = days.length > 10 ? 5 : 1;
  return (
    <svg direction="ltr" role="img" aria-label={`مصرف روزانه‌ی ${faDigits(days.length)} روز اخیر`} viewBox={`0 0 ${W} ${H}`} className="block h-auto w-full">
      {grid.map((g) => (
        <g key={g}>
          <line x1={pad} x2={W} y1={y(g * unit)} y2={y(g * unit)} className="stroke-hair" strokeWidth={1} />
          <text x={0} y={y(g * unit) + 4} fontSize={11} textAnchor="start" direction="ltr" className="fill-ink-2 font-num">
            {Number(g.toFixed(1))} {unit === 1024 ** 3 ? "GB" : "MB"}
          </text>
        </g>
      ))}
      {days.map((d, i) => {
        // Right to left: today on the left edge, as the design reads in RTL.
        const x = W - (i + 1) * bw + 3;
        const h = Math.max(0, H - 24 - y(d.bytes));
        const peak = d.bytes === max && d.bytes > 0;
        return (
          <g key={d.day}>
            <rect x={x} y={H - 24 - h} width={Math.max(bw - 6, 2)} height={h} rx={4} className={peak ? "fill-action" : "fill-link"}>
              <title>{`${dayLabel(d.day)}: ${sizeFa(d.bytes).value} ${sizeFa(d.bytes).unit}`}</title>
            </rect>
            {i % every === 0 && (
              <text x={W - (i + 0.5) * bw} y={H - 6} fontSize={11} textAnchor="middle" className="fill-ink-2">
                {dayLabel(d.day)}
              </text>
            )}
          </g>
        );
      })}
    </svg>
  );
}

/** Desktop-Usage: this computer's own VPN traffic, or a service's (all its devices). */
export function Usage() {
  const { view } = useTunnel();
  const [params] = useSearchParams();
  const services = (view?.sources ?? []).flatMap((s) => (s.kind === "account" ? [s] : []));
  const [source, setSource] = useState<string>(params.get("service") ?? "here");
  const [range, setRange] = useState<"7" | "30">("30");
  const [days, setDays] = useState<DayUsage[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const service = services.find((s) => s.subscriptionId === source) ?? null;

  useEffect(() => {
    setDays(null);
    setError(null);
    const n = Number(range);
    // Newest first for the chart's right-to-left order.
    const done = (d: DayUsage[]) => setDays([...d].reverse());
    if (source === "here") {
      usage.local(n).then(done, (e) => setError(errorText(e)));
    } else {
      usage.service(source, n).then(
        (d) => done(d.map((x) => ({ day: x.day, bytes: x.usedMib * 1024 * 1024 }))),
        (e) => setError(errorText(e)),
      );
    }
  }, [source, range]);

  const sum = useMemo(() => usageSummary(days ?? []), [days]);
  const remaining = service && service.quotaGib !== null ? Math.max(0, service.quotaGib - service.usedGib) * 1024 ** 3 : undefined;

  return (
    <div className="flex min-w-0 flex-1 flex-col gap-3.5">
      <PageHeader
        title="مصرف داده"
        subtitle={source === "here" ? "تاریخچه‌ی این کامپیوتر روی همین‌جا ساخته می‌شود و ۴۵ روز می‌ماند" : "از سرور: همه‌ی دستگاه‌های این سرویس با هم"}
      />
      {services.length > 0 && (
        <div role="radiogroup" aria-label="مصرف کدام" className="flex flex-wrap gap-2">
          {[{ id: "here", name: "این کامپیوتر" }, ...services.map((s) => ({ id: s.subscriptionId, name: s.name }))].map((o) => (
            <button
              key={o.id}
              type="button"
              role="radio"
              aria-checked={source === o.id}
              onClick={() => setSource(o.id)}
              className={`h-9 rounded-xl px-3.5 text-[13px] font-bold ${source === o.id ? "bg-white text-[#062845]" : "glass-clear text-white"}`}
            >
              {o.name}
            </button>
          ))}
        </div>
      )}
      <div className="flex gap-4">
        <Stat label={`مجموع ${faDigits(range)} روز`} bytes={sum.total} />
        <Stat label="میانگین روزانه" bytes={sum.average} />
        <Stat label="پرمصرف‌ترین روز" sub={sum.peak ? dayLabel(sum.peak.day) : "—"} />
        {remaining !== undefined ? (
          <Stat label="باقی‌مانده‌ی سرویس" bytes={remaining} sub={`از ${faDigits(service!.quotaGib!)} گیگ`} />
        ) : (
          <Stat label="پرمصرف‌ترین روز (حجم)" bytes={sum.peak?.bytes ?? 0} />
        )}
      </div>
      <Card padding={18}>
        <CardTitle
          title="مصرف روزانه"
          subtitle={source === "here" ? "ترافیک VPN همین کامپیوتر · سهم دستگاه‌های دیگر در آن نیست" : "روزها به وقت تهران"}
          actions={
            <div className="w-[180px]">
              <Segmented label="بازه" options={RANGES} value={range} onChange={setRange} height={38} />
            </div>
          }
        />
        {error ? (
          <p role="alert" className="m-0 text-[13px] text-bad">
            {error}
          </p>
        ) : days === null ? (
          <p className="m-0 text-[13px] text-ink-2">در حال خواندن…</p>
        ) : sum.total === 0 ? (
          <p className="m-0 p-6 text-center text-[13px] leading-[1.9] text-ink-2">
            {source === "here" ? "در این بازه از این کامپیوتر ترافیکی از VPN رد نشده." : "در این بازه مصرفی برای این سرویس ثبت نشده."}
          </p>
        ) : (
          <Chart days={days} />
        )}
      </Card>
    </div>
  );
}
