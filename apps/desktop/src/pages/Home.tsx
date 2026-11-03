import { useEffect, useState } from "react";
import { Link } from "react-router";
import { Icon, type IconName } from "../design-system/Icon";
import { Logo } from "../design-system/Logo";
import { Button, CountryBadge, PingBars } from "../design-system/controls";
import { useAuth } from "../lib/AuthContext";
import { faDigits } from "../lib/fa";
import { countryCode, daysLeft, elapsed, plainName, speed } from "../lib/format";
import { HISTORY, useTunnel } from "../lib/TunnelContext";
import { ServerPanel } from "./ServerList";

function Stat({ icon, label, value, unit }: { icon: IconName; label: string; value: string; unit?: string }) {
  return (
    <div className="flex flex-1 basis-0 flex-col gap-1 px-3.5">
      <span className="flex items-center gap-1.5 text-xs opacity-85">
        <Icon name={icon} size={14} stroke={2.2} />
        {label}
      </span>
      {unit ? (
        <span dir="ltr" className="text-right font-num text-lg font-bold">
          {value} <span className="text-[11px] opacity-80">{unit}</span>
        </span>
      ) : (
        <span className="text-[17px] font-extrabold">{value}</span>
      )}
    </div>
  );
}

function useNow(active: boolean): number {
  const [now, setNow] = useState(Date.now);
  useEffect(() => {
    if (!active) return;
    const t = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(t);
  }, [active]);
  return now;
}

/** The last minute of download (white) and upload (light blue), as in Desktop-Home. */
function SpeedGraph() {
  const { history } = useTunnel();
  const max = Math.max(1, ...history.map((h) => Math.max(h.downBps, h.upBps)));
  const pts = (pick: (h: (typeof history)[number]) => number) =>
    history.map((h, i) => `${(i + HISTORY - history.length) * (600 / (HISTORY - 1))},${54 - (pick(h) / max) * 50}`).join(" ");
  return (
    <div className="flex flex-col gap-1.5 px-1">
      <div className="flex justify-between text-xs">
        <span className="font-bold">سرعت زنده · ۶۰ ثانیه‌ی اخیر</span>
        <span className="flex gap-3 opacity-90">
          <span className="flex items-center gap-1.5">
            <span className="h-[3px] w-2.5 rounded-sm bg-white" />
            دانلود
          </span>
          <span className="flex items-center gap-1.5">
            <span className="h-[3px] w-2.5 rounded-sm bg-[#7DD8FF]" />
            آپلود
          </span>
        </span>
      </div>
      <svg role="img" aria-label="نمودار سرعت زنده" viewBox="0 0 600 56" preserveAspectRatio="none" className="block h-14 w-full">
        {history.length > 1 && (
          <>
            <polyline points={pts((h) => h.downBps)} fill="none" stroke="#FFFFFF" strokeWidth="2" vectorEffect="non-scaling-stroke" />
            <polyline points={pts((h) => h.upBps)} fill="none" stroke="#7DD8FF" strokeWidth="2" vectorEffect="non-scaling-stroke" />
          </>
        )}
      </svg>
    </div>
  );
}

/** Desktop-Home and Desktop-Home-Connecting, driven by the real tunnel state. */
export function Home() {
  const { view: auth, showSignIn } = useAuth();
  const { view, state, stats, busy, toggle, disconnect } = useTunnel();
  const on = state.status === "on";
  const connecting = state.status === "connecting";
  const now = useNow(on);
  const user = auth?.user ?? null;

  const serverId = state.status === "on" || state.status === "connecting" ? state.serverId : view?.selected;
  const server = view?.servers.find((s) => s.id === serverId) ?? null;
  const account = view?.sources.find((s) => s.kind === "account" && s.id === server?.sourceId) ?? view?.sources.find((s) => s.kind === "account");
  const quota = account?.kind === "account" ? account : null;
  const left = quota ? daysLeft(quota.expiresAt, now) : null;
  const down = speed(stats?.downBps ?? 0);
  const up = speed(stats?.upBps ?? 0);
  const noServers = !view?.servers.length;

  let title: string;
  let subtitle: string;
  if (on) {
    title = elapsed(state.sinceMs, now);
    subtitle = "برای قطع اتصال، روی عینک بزن";
  } else if (connecting && state.stage === "findingIp") {
    title = "پیدا کردن IP تمیز…";
    subtitle = "این شبکه هنوز IP تمیز ندارد؛ یک اسکن کوتاه (تا ۱۵ ثانیه)";
  } else if (connecting && state.stage === "testing") {
    title = "تست سرورها…";
    subtitle = "سریع‌ترین سرور و IP برای این شبکه";
  } else if (connecting) {
    title = state.of > 1 ? `تلاش ${faDigits(state.attempt)} از ${faDigits(state.of)}` : "در حال اتصال…";
    subtitle = "برای لغو، دوباره روی عینک بزن";
  } else if (noServers) {
    title = "هنوز سروری نداری";
    subtitle = "اول یک سرویس بخر یا لینک اشتراکت را در «سرویس‌ها» اضافه کن.";
  } else {
    title = view?.autoSelect ? "سرور: خودکار" : plainName(server?.name ?? "") || "آماده‌ی اتصال";
    subtitle = "برای اتصال، روی عینک بزن";
  }

  return (
    <>
      <div className="flex min-w-0 flex-1 basis-0 flex-col gap-4 max-[1200px]:gap-3">
        <header className="flex items-center gap-3">
          <div className="flex flex-1 flex-col gap-0.5">
            <span dir="ltr" className="text-right font-num text-[22px] font-bold tracking-[-0.5px]">
              GeekVPN
            </span>
            <span className="text-[13px] opacity-90">{user ? `سلام، ${user.displayName}` : "خوش اومدی"}</span>
          </div>
          {!user && (
            <button type="button" onClick={showSignIn} className="glass-clear flex h-10 items-center gap-2 rounded-[13px] px-3.5 text-[13px] font-bold">
              <Icon name="plane" size={16} />
              ورود با تلگرام
            </button>
          )}
          <Link to="/shop" className="flex h-10 items-center gap-2.5 rounded-[13px] bg-pay-bar pe-1.5 ps-3.5 text-[13px] font-bold text-white no-underline">
            خرید سرویس
            <span className="flex h-7 w-7 items-center justify-center rounded-[9px] bg-white text-[#062845]">
              <Icon name="plus" size={16} stroke={2.6} />
            </span>
          </Link>
        </header>

        <div className="flex flex-1 items-center gap-9 px-3 max-[1200px]:flex-col max-[1200px]:items-stretch max-[1200px]:gap-6">
          <div className="relative flex h-[280px] w-[280px] shrink-0 items-center justify-center">
            <svg aria-hidden="true" width="280" height="280" viewBox="0 0 280 280" className="absolute inset-0">
              <circle cx="140" cy="140" r="128" fill="none" stroke="rgba(255,255,255,0.22)" strokeWidth="6" />
              {(on || connecting) && (
                <circle
                  cx="140"
                  cy="140"
                  r="128"
                  fill="none"
                  stroke="#00ACFE"
                  strokeWidth="6"
                  strokeLinecap="round"
                  strokeDasharray={on ? "805 805" : "300 805"}
                  transform="rotate(90 140 140)"
                  className={connecting ? "origin-center animate-spin [animation-duration:2.4s]" : ""}
                />
              )}
            </svg>
            <button
              type="button"
              aria-label={on ? "قطع اتصال" : connecting ? "لغو اتصال" : "اتصال"}
              disabled={busy && !connecting}
              onClick={toggle}
              className={`relative flex h-[236px] w-[236px] items-center justify-center rounded-full transition-transform active:scale-[0.98] ${
                on ? "glass-milk shadow-[0_0_0_14px_rgba(255,255,255,0.12),0_24px_50px_rgba(2,36,84,0.35)]" : "glass-clear shadow-[0_0_0_14px_rgba(255,255,255,0.08),0_24px_50px_rgba(2,36,84,0.3)]"
              }`}
            >
              <span className={connecting ? "opacity-70" : ""}>
                <Logo size={132} color={on ? "#00ACFE" : "#FFFFFF"} />
              </span>
            </button>
          </div>
          <div className="flex min-w-0 flex-1 flex-col gap-3.5">
            <span className="flex items-center gap-2 text-[15px] font-bold">
              <span
                className={`h-[9px] w-[9px] rounded-[2px] ${on ? "bg-ok-bright" : connecting ? "bg-[#FFB547]" : state.status === "failed" ? "bg-bad" : "bg-white/60"}`}
              />
              {on ? "متصل و امن" : connecting ? "در حال اتصال" : state.status === "failed" ? "وصل نشد" : "قطع"}
            </span>
            <span dir={on ? "ltr" : undefined} className={`leading-none ${on ? "text-right font-num text-[56px] font-bold tracking-[1px]" : "text-[28px] font-extrabold leading-tight"}`}>
              {title}
            </span>
            <span className="text-[13px] leading-[1.8] opacity-90" role="status">
              {state.status === "failed" ? state.message : subtitle}
            </span>
            {on && state.note && <span className="rounded-xl bg-white/15 px-3 py-2 text-xs leading-[1.8]">{state.note}</span>}
            {state.status === "failed" && state.blocking && (
              <span className="flex">
                <Button kind="white" icon="power" height={40} disabled={busy} onClick={disconnect}>
                  قطع و باز کردن اینترنت
                </Button>
              </span>
            )}
            {server ? (
              <Link to="/servers" className="glass-milk mt-1.5 flex max-w-[380px] items-center gap-3 rounded-[20px] py-2.5 pe-3.5 ps-2.5 no-underline">
                <CountryBadge code={countryCode(server.name)} selected size={44} />
                <span className="flex min-w-0 flex-1 flex-col gap-0.5">
                  <span className="text-xs text-ink-2">
                    سرور · {view?.route === "smart" ? "مسیر هوشمند" : view?.route === "global" ? "مسیر سراسری" : "مسیر مستقیم"}
                  </span>
                  <span className="truncate text-[15px] font-extrabold text-ink">{plainName(server.name) || server.address}</span>
                </span>
                <PingBars ms={on ? state.delayMs : (server.delayMs ?? 0)} />
                <span className="flex text-muted">
                  <Icon name="chev" size={18} />
                </span>
              </Link>
            ) : (
              noServers && (
                <Link to="/services" className="glass-milk mt-1.5 flex max-w-[380px] items-center gap-3 rounded-[20px] py-2.5 pe-3.5 ps-2.5 no-underline">
                  <span className="flex h-11 w-11 shrink-0 items-center justify-center rounded-[14px] bg-soft text-link">
                    <Icon name="link" size={20} />
                  </span>
                  <span className="flex flex-1 flex-col gap-0.5">
                    <span className="text-xs text-ink-2">سرویس‌ها</span>
                    <span className="text-[15px] font-extrabold text-ink">افزودن لینک اشتراک</span>
                  </span>
                  <span className="flex text-muted">
                    <Icon name="chev" size={18} />
                  </span>
                </Link>
              )
            )}
          </div>
        </div>

        <div className="glass-clear flex rounded-[20px] px-1 py-3.5">
          <Stat icon="dl" label="دانلود" value={on ? down.value : "—"} unit={on ? down.unit : undefined} />
          <span aria-hidden="true" className="w-px self-stretch bg-white/25" />
          <Stat icon="up" label="آپلود" value={on ? up.value : "—"} unit={on ? up.unit : undefined} />
          <span aria-hidden="true" className="w-px self-stretch bg-white/25" />
          <Stat icon="bolt" label="تأخیر" value={on ? `${state.delayMs}` : "—"} unit={on ? "ms" : undefined} />
          <span aria-hidden="true" className="w-px self-stretch bg-white/25" />
          {on && state.mode === "tun" ? (
            <Stat icon="shield" label="حالت" value="TUN" unit={state.killSwitch ? "Kill Switch" : undefined} />
          ) : (
            <Stat icon="shield" label="پروکسی محلی" value={on ? `${state.httpPort}` : "—"} unit={on ? "HTTP" : undefined} />
          )}
          <span aria-hidden="true" className="w-px self-stretch bg-white/25" />
          <Stat icon="clock" label="زمان باقی‌مانده" value={left === null ? "—" : `${faDigits(left)} روز`} />
        </div>

        {on && <SpeedGraph />}

        {quota && quota.quotaGib !== null && (
          <div className="flex flex-col gap-2 px-1">
            <div className="flex justify-between text-xs">
              <span className="font-bold">
                {account?.name} — {faDigits(quota.usedGib.toFixed(1))} از {faDigits(quota.quotaGib)} گیگ
              </span>
              <span className="opacity-90">{faDigits(Math.round((quota.usedGib / quota.quotaGib) * 100))}٪</span>
            </div>
            <div className="flex h-2.5 gap-[3px]" aria-hidden="true">
              {Array.from({ length: 40 }, (_, i) => (
                <span key={i} className={`flex-1 rounded-[3px] ${i < Math.round((quota.usedGib / (quota.quotaGib ?? 1)) * 40) ? "bg-white" : "bg-white/22"}`} />
              ))}
            </div>
          </div>
        )}
      </div>

      <ServerPanel />
    </>
  );
}
