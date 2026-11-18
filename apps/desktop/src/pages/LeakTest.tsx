import { useState } from "react";
import { Button } from "../design-system/controls";
import { CardTitle } from "../design-system/layout";
import { errorText } from "../lib/auth";
import { countryFa, flagFor } from "../lib/format";
import { leakTest, type LeakReport, type Verdict } from "../lib/leak";
import { useTunnel } from "../lib/TunnelContext";

const TITLE = { ip: "IP", ipv6: "IPv6", dns: "DNS" } as const;
const MARK: Record<Verdict, { text: string; tone: string }> = {
  ok: { text: "امن", tone: "bg-ok-soft text-ok" },
  leak: { text: "نشت", tone: "bg-bad-soft text-bad" },
  unknown: { text: "نامشخص", tone: "bg-white/10 text-ink-2" },
};

function Place({ ip, country }: { ip: string; country: string }) {
  return (
    <span className="flex items-center gap-2">
      <span aria-hidden="true">{flagFor(country)}</span>
      <span dir="ltr" className="font-num text-sm font-bold select-text">
        {ip}
      </span>
      {country && <span className="text-[11px] text-ink-2">{countryFa(country)}</span>}
    </span>
  );
}

/** «تست نشت»: IP, IPv6 and DNS, measured on the running connection. */
export function LeakTest() {
  const { state } = useTunnel();
  const [report, setReport] = useState<LeakReport | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const on = state.status === "on";

  const run = async () => {
    setBusy(true);
    setError(null);
    try {
      setReport(await leakTest());
    } catch (e) {
      setError(errorText(e));
    }
    setBusy(false);
  };

  return (
    <section className="glass-milk flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto rounded-[26px] p-5 text-ink [scrollbar-width:thin]">
      <CardTitle
        micro="Leak test"
        title="تست نشت IP و DNS"
        subtitle="آیا همه‌چیز واقعاً از VPN می‌رود؟"
        actions={
          <Button kind="white" icon="radar" height={40} disabled={!on || busy} onClick={() => void run()}>
            {busy ? "در حال تست… (تا ۲۰ ثانیه)" : "شروع تست"}
          </Button>
        }
      />
      {!on && <span className="text-[13px] text-ink-2">اول وصل شو؛ تست روی اتصال روشن انجام می‌شود.</span>}
      {error && <span role="alert" className="text-[13px] text-bad">{error}</span>}
      {report && (
        <>
          <div className="flex flex-col border-y border-hair">
            {report.checks.map((c) => (
              <div key={c.key} className="flex items-start gap-3 border-b border-hair py-3 last:border-b-0">
                <span dir="ltr" className="w-[44px] shrink-0 pt-0.5 text-right font-num text-[11px] tracking-[0.16em] text-ink-2">
                  {TITLE[c.key]}
                </span>
                <span className="min-w-0 flex-1 text-[13px] leading-[1.8]">{c.note}</span>
                <span className={`shrink-0 rounded-full px-2.5 py-1 text-[11px] font-bold ${MARK[c.verdict].tone}`}>{MARK[c.verdict].text}</span>
              </div>
            ))}
          </div>
          <div className="grid grid-cols-2 gap-x-8 gap-y-3 max-[1100px]:grid-cols-1">
            {report.vpn && (
              <div className="flex flex-col gap-1">
                <span className="text-[11px] text-ink-2">IP از داخل VPN</span>
                <Place ip={report.vpn.ip} country={report.vpn.country} />
              </div>
            )}
            {report.real && (
              <div className="flex flex-col gap-1">
                <span className="text-[11px] text-ink-2">IP بدون پروکسی (برنامه‌هایی که پروکسی را نمی‌خوانند)</span>
                <Place ip={report.real.ip} country={report.real.country} />
              </div>
            )}
            {report.ipv6 && (
              <div className="flex flex-col gap-1">
                <span className="text-[11px] text-ink-2">IPv6</span>
                <Place ip={report.ipv6.ip} country={report.ipv6.country} />
              </div>
            )}
          </div>
          {report.resolvers.length > 0 && (
            <div className="flex flex-col gap-1">
              <span className="micro text-[9px] tracking-[0.22em]">DNS servers seen</span>
              <div className="flex flex-col border-t border-hair">
                {report.resolvers.map((r) => (
                  <div key={r.ip} className="flex items-center gap-3 border-b border-hair py-2">
                    <Place ip={r.ip} country={r.country} />
                    <span dir="ltr" className="min-w-0 flex-1 truncate text-left font-num text-[11px] text-ink-2">
                      {r.asn}
                    </span>
                  </div>
                ))}
              </div>
            </div>
          )}
        </>
      )}
      <span className="text-[11px] leading-[1.8] text-ink-2">
        IP با <span dir="ltr" className="font-num">cloudflare.com/cdn-cgi/trace</span> و DNS با سرویس <span dir="ltr" className="font-num">bash.ws</span> سنجیده می‌شود (فقط در حالت TUN). چیزی جز همین درخواست‌های تست فرستاده نمی‌شود.
      </span>
    </section>
  );
}
