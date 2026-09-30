import { useCallback, useEffect, useState } from "react";
import { Badge, Button, ClearChips, Switch } from "../design-system/controls";
import { Card, CardTitle, EmptyState, PageHeader } from "../design-system/layout";
import { errorText } from "../lib/auth";
import { faDigits } from "../lib/fa";
import { plainName } from "../lib/format";
import { inTauri } from "../lib/platform";
import { scanner, tools, type ScanView, type SpeedResult } from "../lib/servers";
import { useTunnel } from "../lib/TunnelContext";
import { ServerPanel } from "./ServerList";

type Tab = "scan" | "delay" | "log";

export function Tools() {
  const [tab, setTab] = useState<Tab>("scan");
  return (
    <div className="flex min-w-0 flex-1 flex-col gap-3.5">
      <PageHeader title="ابزارها" subtitle="اسکنر IP تمیز، تست سرعت و لاگ هسته" />
      <ClearChips
        options={[
          { value: "scan", label: "اسکنر و تست سرعت" },
          { value: "delay", label: "تست تأخیر سرورها" },
          { value: "log", label: "لاگ هسته" },
        ]}
        value={tab}
        onChange={setTab}
      />
      {tab === "scan" && (
        <div className="flex min-h-0 flex-1 gap-4">
          <Scanner />
          <SpeedTest />
        </div>
      )}
      {tab === "delay" && (
        <div className="flex min-h-0 flex-1">
          <ServerPanel />
        </div>
      )}
      {tab === "log" && <CoreLog />}
    </div>
  );
}

/** «بهینه‌ساز کلادفلر» (Desktop-Tools). */
function Scanner() {
  const { view } = useTunnel();
  const [scan, setScan] = useState<ScanView | null>(null);
  const [serverId, setServerId] = useState<string | undefined>();
  const [progress, setProgress] = useState<{ tested: number; total: number; found: number } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [note, setNote] = useState<string | null>(null);

  const reload = useCallback(() => {
    if (inTauri) void scanner.state(serverId).then(setScan, (e) => setError(errorText(e)));
  }, [serverId]);
  useEffect(reload, [reload, view?.selected]);

  useEffect(() => {
    const off = scanner.onEvent((e) => {
      if (e.kind === "progress") setProgress({ tested: e.tested, total: e.total, found: e.found });
      if (e.kind === "result") setScan((s) => s && { ...s, results: [...s.results, e.result].sort((a, b) => a.latencyMs - b.latencyMs) });
      if (e.kind === "finish") {
        setProgress(null);
        if (e.error) setError(e.error);
        reload();
      }
    });
    return () => void off.then((f) => f());
  }, [reload]);

  if (!scan) return <section className="glass-milk flex-1 rounded-[26px]" />;
  if (scan.candidates.length === 0) {
    return (
      <section className="glass-clear flex flex-1 flex-col rounded-[26px]">
        <EmptyState
          icon="radar"
          title="سرویسی برای اسکن نیست"
          text="اسکنر IP تمیز برای سرویس‌های «مستقیم» و لینک‌هایی است که از کلادفلر رد می‌شوند (ws، grpc، xhttp یا httpupgrade با TLS و دامنه). سرویس‌های تونل و الیت به سرورهای خود ما وصل‌اند و IP کلادفلر به کارشان نمی‌آید."
        />
      </section>
    );
  }

  const current = scan.candidates.find((c) => c.id === scan.serverId);
  const running = scan.running || progress !== null;
  const pct = progress && progress.total ? Math.round((progress.tested / progress.total) * 100) : 0;

  const start = async () => {
    if (!scan.serverId) return;
    setError(null);
    setNote(null);
    setScan({ ...scan, results: [], running: true });
    setProgress({ tested: 0, total: 0, found: 0 });
    try {
      const applied = await scanner.start(scan.serverId, scan.downloadTest);
      setNote(
        applied
          ? `بهترین IP (${applied}) اعمال شد؛ از اتصال بعدی روی این شبکه استفاده می‌شود.`
          : "روی این شبکه IP تمیزی جواب نداد؛ اتصال با آدرس خود سرور می‌ماند.",
      );
    } catch (e) {
      setError(errorText(e));
      setProgress(null);
    }
    reload();
  };
  const use = async (ip: string | null) => {
    if (!scan.serverId) return;
    try {
      await scanner.use(scan.serverId, ip);
      setNote(ip ? `از این به بعد روی این شبکه از ${ip} استفاده می‌شود.` : "به IP اصلی سرور برگشت.");
    } catch (e) {
      setError(errorText(e));
    }
    reload();
  };

  const cols = "grid grid-cols-[minmax(0,1.5fr)_70px_80px_70px_60px_90px_100px] items-center gap-2";
  return (
    <section className="glass-milk flex min-w-0 flex-1 flex-col overflow-hidden rounded-[26px] text-ink">
      <div className="flex flex-col gap-3 border-b border-hair p-4">
        <CardTitle
          title="بهینه‌ساز کلادفلر"
          subtitle="اسکن IP تمیز برای سرویس‌های مستقیم پشت CDN"
          actions={
            running ? (
              <Button kind="danger" icon="x" height={40} onClick={() => void scanner.stop()}>
                توقف
              </Button>
            ) : (
              <Button kind="action" icon="radar" height={40} onClick={() => void start()}>
                {scan.results.length ? "اسکن دوباره" : "شروع اسکن"}
              </Button>
            )
          }
        />
        {scan.candidates.length > 1 && (
          <label className="flex items-center gap-2 text-xs text-ink-2">
            سرور:
            <select
              value={scan.serverId ?? ""}
              disabled={running}
              onChange={(e) => setServerId(e.target.value)}
              className="h-9 min-w-0 flex-1 rounded-xl border border-chip-border bg-chip px-2 text-[13px] text-ink"
            >
              {scan.candidates.map((c) => (
                <option key={c.id} value={c.id}>
                  {plainName(c.name) || c.sni}
                </option>
              ))}
            </select>
          </label>
        )}
        <div className="flex flex-col gap-1.5">
          <span className="flex justify-between text-xs text-ink-2">
            <span>
              {progress && !progress.total
                ? "بررسی دامنه و آماده‌سازی…"
                : progress
                ? `${faDigits(progress.tested)} از ${faDigits(progress.total)} آدرس · ${faDigits(progress.found)} سالم`
                : scan.scannedAt
                  ? `${faDigits(scan.results.length)} IP تمیز · ${new Date(scan.scannedAt).toLocaleString("fa-IR")}`
                  : "هنوز روی این شبکه اسکن نشده"}
            </span>
            <span dir="ltr" className="font-num">
              SNI: {current?.sni}
            </span>
          </span>
          <span className="h-2 overflow-hidden rounded bg-track">
            <span className="block h-full bg-link transition-[width]" style={{ width: `${progress ? pct : scan.results.length ? 100 : 0}%` }} />
          </span>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <Badge tone="link">شبکه: {scan.network.label}</Badge>
          {scan.behindCloudflare === true && <Badge tone="ok">دامنه تأییدشده در رنج کلادفلر</Badge>}
          {scan.behindCloudflare === false && <Badge tone="bad">دامنه پشت کلادفلر نیست</Badge>}
          <span className="flex-1" />
          <span className="text-xs text-ink-2">تست دانلود</span>
          <Switch
            label="تست دانلود"
            checked={scan.downloadTest}
            onChange={(v) => {
              setScan({ ...scan, downloadTest: v });
              void scanner.setDownload(v);
            }}
          />
        </div>
      </div>
      <div className={`${cols} border-b border-hair px-4 py-2.5 text-xs font-bold text-ink-2`}>
        <span>IP</span>
        <span>پینگ</span>
        <span>تأخیر</span>
        <span>لرزش</span>
        <span>colo</span>
        <span>دانلود</span>
        <span />
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto">
        {scan.results.length === 0 && (
          <p className="m-0 p-6 text-center text-[13px] leading-[1.9] text-ink-2">
            {running ? "در حال پیدا کردن…" : "با «شروع اسکن» تا ۳۰۰ آدرس کلادفلر امتحان می‌شود و با پیدا شدن ۵ آدرس سالم تمام می‌شود."}
          </p>
        )}
        {scan.results.map((r) => {
          const inUse = r.ip === scan.inUse;
          return (
            <div key={r.ip} className={`${cols} border-b border-hair px-4 py-2 font-num text-[13px]`}>
              <span dir="ltr" className="text-right font-bold">
                {r.ip}
              </span>
              <span>{r.pingMs}ms</span>
              <span>{r.latencyMs}ms</span>
              <span>{r.jitterMs.toFixed(1)}ms</span>
              <span>{r.colo ?? "—"}</span>
              <span>{r.downloadKBps ? `${(r.downloadKBps / 1024).toFixed(1)} MB/s` : "—"}</span>
              <button
                type="button"
                disabled={inUse || running}
                onClick={() => void use(r.ip)}
                className={`h-[30px] rounded-[10px] text-xs font-bold ${inUse ? "bg-action text-on-action" : "bg-soft text-link"}`}
              >
                {inUse ? "در حال استفاده" : "استفاده"}
              </button>
            </div>
          );
        })}
      </div>
      <div className="flex items-center gap-2 px-4 py-3">
        <Button kind="soft" icon="refresh" height={40} disabled={!scan.inUse || running} onClick={() => void use(null)}>
          بازگشت به IP اصلی
        </Button>
        <span className="flex-1" />
        <span role="status" className={`text-xs ${error ? "text-bad" : "text-ink-2"}`}>
          {error ?? note ?? (scan.inUse ? `IP در حال استفاده روی این شبکه: ${scan.inUse}` : "")}
        </span>
      </div>
    </section>
  );
}

/** A 240° arc, `p` of it filled. */
function arc(p: number, r = 92, c = 110): string {
  const a0 = (150 * Math.PI) / 180;
  const a1 = ((150 + 240 * Math.min(Math.max(p, 0), 1)) * Math.PI) / 180;
  const large = 240 * p > 180 ? 1 : 0;
  return `M${c + r * Math.cos(a0)} ${c + r * Math.sin(a0)} A${r} ${r} 0 ${large} 1 ${c + r * Math.cos(a1)} ${c + r * Math.sin(a1)}`;
}

/** «تست سرعت» (Desktop-Tools). */
function SpeedTest() {
  const { state } = useTunnel();
  const [running, setRunning] = useState(false);
  const [phase, setPhase] = useState<"ping" | "download" | "upload" | null>(null);
  const [live, setLive] = useState(0);
  const [result, setResult] = useState<SpeedResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const off = tools.onSpeed((p) => {
      setPhase(p.phase);
      setLive(p.mbps);
    });
    return () => void off.then((f) => f());
  }, []);

  const run = async () => {
    setRunning(true);
    setError(null);
    setResult(null);
    setLive(0);
    try {
      setResult(await tools.speedTest());
    } catch (e) {
      setError(errorText(e));
    } finally {
      setRunning(false);
      setPhase(null);
    }
  };

  const shown = running ? live : (result?.downloadMbps ?? 0);
  // The gauge tops out at 100 Mb/s; beyond that it is simply full.
  const fill = Math.min(shown / 100, 1);
  const label = running ? (phase === "upload" ? "آپلود" : phase === "download" ? "دانلود" : "پینگ") : "دانلود";
  const via = state.status === "on" ? "از داخل تونل فعلی" : "شبکه‌ی خودت، بدون VPN";

  return (
    <Card className="w-[330px] shrink-0" padding={18}>
      <CardTitle title="تست سرعت" subtitle={via} />
      <div className="relative h-[190px] w-[220px] self-center">
        <svg aria-hidden="true" width="220" height="220" viewBox="0 0 220 220" className="absolute right-0 top-0">
          <path d={arc(1)} fill="none" stroke="var(--color-track)" strokeWidth="14" strokeLinecap="round" />
          {fill > 0 && <path d={arc(fill)} fill="none" stroke="var(--color-link)" strokeWidth="14" strokeLinecap="round" />}
        </svg>
        <div className="absolute inset-x-0 top-[70px] flex flex-col items-center">
          <span dir="ltr" className="font-num text-[40px] font-bold leading-none">
            {shown ? shown.toFixed(1) : "—"}
          </span>
          <span className="text-xs text-ink-2">مگابیت بر ثانیه · {label}</span>
        </div>
      </div>
      <div className="flex gap-2.5 text-center">
        {[
          ["پینگ", result?.pingMs, "ms"],
          ["لرزش", result?.jitterMs, "ms"],
          ["آپلود", result?.uploadMbps != null ? result.uploadMbps.toFixed(1) : null, "Mb/s"],
        ].map(([k, v, u]) => (
          <div key={k as string} className="flex flex-1 flex-col gap-0.5 rounded-[14px] bg-soft-button p-2">
            <span className="text-[11px] text-ink-2">{k}</span>
            <span dir="ltr" className="font-num text-lg font-bold">
              {v ?? "—"}
              {v != null && <span className="text-[11px] font-normal"> {u}</span>}
            </span>
          </div>
        ))}
      </div>
      <div className="flex gap-1.5">
        <Badge tone={phase === "ping" || result ? "ok" : "link"}>پینگ</Badge>
        <Badge tone={phase === "download" ? "warn" : result?.downloadMbps != null ? "ok" : "link"}>دانلود</Badge>
        <Badge tone={phase === "upload" ? "warn" : result?.uploadMbps != null ? "ok" : "link"}>آپلود</Badge>
      </div>
      {running ? (
        <Button kind="danger" icon="x" height={46} onClick={() => void tools.cancelSpeed()}>
          توقف
        </Button>
      ) : (
        <Button kind="action" icon="gauge" height={46} onClick={() => void run()}>
          {result ? "شروع دوباره" : "شروع تست"}
        </Button>
      )}
      {error && (
        <span role="alert" className="text-xs text-bad">
          {error}
        </span>
      )}
      <span className="text-[11px] leading-[1.8] text-ink-2">هر مرحله حداکثر ۱۰ ثانیه (دانلود تا ۵۰، آپلود تا ۲۰ مگابایت) از سرورهای Cloudflare.</span>
    </Card>
  );
}

/** «لاگ هسته»: geekcore's last lines. */
function CoreLog() {
  const [log, setLog] = useState("");
  const load = useCallback(() => {
    if (inTauri) void tools.coreLog().then(setLog, (e) => setLog(errorText(e)));
  }, []);
  useEffect(load, [load]);
  return (
    <section className="glass-milk flex min-h-0 flex-1 flex-col gap-3 rounded-[26px] p-4 text-ink">
      <CardTitle
        title="لاگ هسته"
        subtitle="آخرین خط‌های Xray؛ برای گزارش مشکل"
        actions={
          <Button kind="soft" icon="refresh" height={38} onClick={load}>
            تازه کردن
          </Button>
        }
      />
      <pre dir="ltr" className="m-0 min-h-0 flex-1 overflow-auto whitespace-pre-wrap rounded-2xl bg-chip p-3 text-left font-num text-[12px] leading-[1.7]">
        {log || "هنوز چیزی ثبت نشده."}
      </pre>
    </section>
  );
}
