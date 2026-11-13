import { useEffect, useState, type ReactNode } from "react";
import { Link } from "react-router";
import { Icon } from "../design-system/Icon";
import { Logo } from "../design-system/Logo";
import { Button, Hair, RuleRow } from "../design-system/controls";
import { useAuth } from "../lib/AuthContext";
import { faDigits } from "../lib/fa";
import { countryFa, daysLeft, elapsed, flagFor, flagOf, plainName, speed } from "../lib/format";
import { exitIp, system, type ExitIp } from "../lib/servers";
import { useTunnel } from "../lib/TunnelContext";
import { ServerMark, ServerPanel } from "./ServerList";

function useNow(active: boolean): number {
  const [now, setNow] = useState(Date.now);
  useEffect(() => {
    if (!active) return;
    const t = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(t);
  }, [active]);
  return now;
}

/** «شب بخیر» and the Latin micro-label that goes with it. */
function greeting(hour: number): { fa: string; en: string } {
  if (hour >= 5 && hour < 12) return { fa: "صبح بخیر", en: "Good morning" };
  if (hour >= 12 && hour < 17) return { fa: "روز بخیر", en: "Good afternoon" };
  if (hour >= 17 && hour < 21) return { fa: "عصر بخیر", en: "Good evening" };
  return { fa: "شب بخیر", en: "Good evening" };
}

/** One figure of the Down / Up / Ping strip. */
function Figure({ label, value, unit }: { label: string; value: string; unit?: string }) {
  return (
    <div className="flex min-w-0 flex-1 flex-col gap-0.5 py-2.5">
      <span className="micro text-[9px] tracking-[0.2em]">{label}</span>
      <span dir="ltr" className="whitespace-nowrap text-right font-num text-xl font-medium">
        {value}
        {unit && <span className="ms-[3px] text-[11px] text-ink-2">{unit}</span>}
      </span>
    </div>
  );
}

/** The light column under the connect ring when connected (Desktop-Home). */
function LightColumn() {
  const steps = [0.56, 0.494, 0.433, 0.377, 0.326, 0.279, 0.238, 0.201, 0.169, 0.141, 0.118, 0.099, 0.084, 0.073, 0.065, 0.061, 0.06];
  return (
    <>
      {steps.map((o, i) => (
        <span
          key={o}
          aria-hidden="true"
          className="pointer-events-none absolute left-0 -z-10 h-[230px] w-[230px] rounded-full bg-light"
          style={{ top: i * 30, opacity: o }}
        />
      ))}
    </>
  );
}

/**
 * The exit address and, in TUN mode, the open connections, for as long as
 * the tunnel is up. The address is asked once per connection and server
 * (again after a failure, and when the window comes back to the front).
 */
function useLiveInfo(key: string | null, tun: boolean) {
  const [exit, setExit] = useState<ExitIp | null>(null);
  const [failed, setFailed] = useState(false);
  const [conns, setConns] = useState<number | null>(null);
  useEffect(() => {
    setExit(null);
    setFailed(false);
    if (!key) return;
    let alive = true;
    let retry: ReturnType<typeof setTimeout> | undefined;
    const ask = () =>
      exitIp().then(
        (e) => alive && (setExit(e), setFailed(false)),
        () => {
          if (!alive) return;
          setFailed(true);
          retry = setTimeout(ask, 15_000);
        },
      );
    // A moment for the system proxy / TUN routes to settle.
    const first = setTimeout(ask, 1500);
    window.addEventListener("focus", ask);
    return () => {
      alive = false;
      clearTimeout(first);
      clearTimeout(retry);
      window.removeEventListener("focus", ask);
    };
  }, [key]);
  useEffect(() => {
    setConns(null);
    if (!key || !tun) return;
    let alive = true;
    const read = () => system.connections().then((c) => alive && setConns(c.available ? c.connections.length : null), () => {});
    read();
    const t = setInterval(read, 5000);
    return () => {
      alive = false;
      clearInterval(t);
    };
  }, [key, tun]);
  return { exit, failed, conns };
}

type Step = { title: string; hint?: string; state: "done" | "now" | "next" };

function StepRow({ step }: { step: Step }) {
  return (
    <div className={`flex items-center gap-3 border-b border-hair py-[9px] ${step.state === "next" ? "opacity-50" : ""}`}>
      {step.state === "done" ? (
        <span className="flex h-6 w-6 items-center justify-center rounded-full bg-white text-navy">
          <Icon name="check" size={14} stroke={3} />
        </span>
      ) : step.state === "now" ? (
        <span className="box-border flex h-6 w-6 items-center justify-center rounded-full border-2 border-blue-text">
          <span className="h-2.5 w-2.5 animate-pulse rounded-full bg-blue-text" />
        </span>
      ) : (
        <span className="box-border h-6 w-6 rounded-full border-[1.5px] border-white/35" />
      )}
      <span className="flex flex-col gap-0.5">
        <span className={`text-sm ${step.state === "now" ? "font-extrabold" : "font-bold"}`}>{step.title}</span>
        {step.hint && <span className="text-xs text-ink-2">{step.hint}</span>}
      </span>
    </div>
  );
}

/** Desktop-Home, Desktop-Home (قطع) and Desktop-Home-Connecting, driven by the real tunnel state. */
export function Home() {
  const { view: auth, showSignIn } = useAuth();
  const { view, state, stats, busy, toggle, disconnect } = useTunnel();
  const on = state.status === "on";
  const connecting = state.status === "connecting";
  const failed = state.status === "failed";
  const now = useNow(on);
  const user = auth?.user ?? null;
  const hello = greeting(new Date(now).getHours());

  const serverId = state.status === "on" || state.status === "connecting" ? state.serverId : view?.selected;
  const server = view?.servers.find((s) => s.id === serverId) ?? null;
  const account = view?.sources.find((s) => s.kind === "account" && s.id === server?.sourceId) ?? view?.sources.find((s) => s.kind === "account");
  const quota = account?.kind === "account" ? account : null;
  const left = quota ? daysLeft(quota.expiresAt, now) : null;
  const down = speed(stats?.downBps ?? 0);
  const up = speed(stats?.upBps ?? 0);
  const noServers = !view?.servers.length;
  const routeFa = view?.route === "smart" ? "مسیر هوشمند" : view?.route === "global" ? "مسیر سراسری" : "مسیر مستقیم";
  const tunKs = view?.mode === "tun" && view.killSwitch;
  const live = useLiveInfo(on ? `${state.sinceMs}/${state.serverId}` : null, on && state.mode === "tun");

  let title: ReactNode;
  let side: ReactNode;
  if (on) {
    title = (
      <>
        متصل و <span className="text-blue-text">امن</span>
      </>
    );
    side = (
      <span dir="ltr" className="font-num text-[44px] font-medium leading-none">
        {elapsed(state.sinceMs, now)}
      </span>
    );
  } else if (connecting) {
    title = (
      <>
        اتصال <span className="text-blue-text">هوشمند</span>
      </>
    );
    side = null;
  } else if (failed) {
    title = "وصل نشد";
    side = <span className="micro text-xs">Status · Failed</span>;
  } else {
    title = noServers ? "هنوز سروری نداری" : "قطع است";
    side = <span className="micro text-xs">Status · Off</span>;
  }

  // Smart connect's stages as the design's checklist.
  let steps: Step[] = [];
  if (connecting) {
    const at = state.stage === "findingIp" ? 0 : state.stage === "testing" ? 1 : 2;
    const mark = (i: number): Step["state"] => (i < at ? "done" : i === at ? "now" : "next");
    steps = [
      { title: "پیدا کردن IP تمیز", hint: state.stage === "findingIp" ? "اسکن کوتاه برای این شبکه (تا ۱۵ ثانیه)" : undefined, state: mark(0) },
      { title: "تست تأخیر سرورها", hint: state.stage === "testing" ? "سریع‌ترین سرور و IP برای این شبکه" : undefined, state: mark(1) },
      ...(tunKs ? [{ title: "تنظیم فایروال و Kill Switch", state: mark(2) } as Step] : []),
      {
        title: state.of > 1 ? `اتصال · تلاش ${faDigits(state.attempt)} از ${faDigits(state.of)}` : "اتصال",
        hint: state.stage === "connecting" && server ? plainName(server.name) || server.address : undefined,
        state: mark(2),
      },
    ];
    const current = steps.findIndex((s) => s.state === "now");
    side = (
      <span className="micro text-xs">
        Step {current + 1} / {steps.length}
      </span>
    );
  }

  const pct = quota && quota.quotaGib ? Math.min(100, Math.round((quota.usedGib / quota.quotaGib) * 100)) : null;

  return (
    <>
      <div className="flex min-w-0 flex-1 flex-col gap-[18px]">
        <div className="flex items-center justify-between gap-3">
          <span className="micro">{hello.en} · Home</span>
          <span className="flex items-center gap-2">
            {!user && (
              <Button kind="soft" icon="plane" height={40} onClick={showSignIn}>
                ورود با تلگرام
              </Button>
            )}
            <Link to="/shop" className="flex h-10 items-center gap-2 whitespace-nowrap rounded-full bg-white px-[18px] text-[13px] font-extrabold text-navy no-underline">
              خرید سرویس
            </Link>
          </span>
        </div>
        <Hair />
        <div className="flex items-end justify-between gap-4">
          <div className="flex min-w-0 flex-col gap-1.5">
            <span className="text-[15px] text-ink-2">
              {hello.fa}، {user ? user.displayName || `کاربر ${faDigits(user.telegramId)}` : "مهمان"}
            </span>
            <h1 className="m-0 text-[56px] font-extrabold leading-[1.2] max-[1200px]:text-[44px]">{title}</h1>
          </div>
          <div className="shrink-0 pb-2">{side}</div>
        </div>

        <div className="flex min-h-0 flex-1 items-center gap-9 overflow-hidden max-[1100px]:flex-col max-[1100px]:items-stretch">
          <div className="relative h-[230px] w-[230px] shrink-0 self-center isolate">
            {on && <LightColumn />}
            <button
              type="button"
              aria-label={on ? "قطع اتصال" : connecting ? "لغو اتصال" : "اتصال"}
              disabled={busy && !connecting}
              onClick={toggle}
              className="glass-milk relative flex h-[230px] w-[230px] items-center justify-center rounded-full shadow-[inset_0_1px_0_rgba(255,255,255,0.45),0_30px_70px_rgba(0,4,30,0.45)] transition-transform active:scale-[0.98]"
            >
              <span
                className={`flex h-[138px] w-[138px] items-center justify-center rounded-full ${
                  on ? "bg-white" : "border border-white/30 bg-white/10"
                } ${connecting ? "animate-pulse" : ""}`}
              >
                <Logo size={on ? 93 : 89} color={on ? "#07123A" : "rgba(255,255,255,0.7)"} />
              </span>
            </button>
          </div>

          <div className="relative z-10 flex min-w-0 flex-1 flex-col gap-3.5">
            {connecting ? (
              <div className="flex flex-col">
                <Hair />
                {steps.map((s) => (
                  <StepRow key={s.title} step={s} />
                ))}
                <span className="pt-2.5 text-xs text-ink-2">برای لغو، دوباره روی عینک بزن</span>
              </div>
            ) : (
              <>
                <div className="flex flex-col">
                  <Hair />
                  <div className="flex gap-3.5">
                    <Figure label="Down" value={on ? down.value : "—"} unit={on ? down.unit : undefined} />
                    <Hair vertical />
                    <Figure label="Up" value={on ? up.value : "—"} unit={on ? up.unit : undefined} />
                    <Hair vertical />
                    <Figure label="Ping" value={on ? String(state.delayMs) : "—"} unit={on ? "ms" : undefined} />
                  </div>
                  <Hair />
                </div>
                <div className="flex flex-col">
                  <Hair />
                  {server ? (
                    <Link to="/servers" className="text-inherit no-underline">
                      <RuleRow
                        mark={view?.autoSelect && !on ? <Icon name="bolt" size={18} /> : flagOf(server.name) ? <ServerMark name={server.name} /> : <Icon name="globe" size={18} />}
                        title={view?.autoSelect && !on ? "خودکار" : <span dir="auto">{plainName(server.name) || server.address}</span>}
                        hint={`سرور · ${routeFa}`}
                        end={<Icon name="chev" size={16} />}
                      />
                    </Link>
                  ) : (
                    <Link to="/services" className="text-inherit no-underline">
                      <RuleRow mark={<Icon name="link" size={18} />} title="افزودن لینک اشتراک" hint="اول یک سرویس بخر یا لینک اشتراکت را اضافه کن" end={<Icon name="chev" size={16} />} />
                    </Link>
                  )}
                  <RuleRow
                    mark={<Icon name="shield" size={18} />}
                    title={(on ? state.mode : view?.mode) === "tun" ? "حالت TUN" : "پروکسی سیستم"}
                    end={
                      on ? (
                        state.mode === "tun" ? (
                          state.killSwitch ? "Kill Switch روشن" : "همه‌ی برنامه‌ها"
                        ) : (
                          <span dir="ltr" className="font-num">HTTP {state.httpPort} · SOCKS {state.socksPort}</span>
                        )
                      ) : (
                        "—"
                      )
                    }
                  />
                  {on && (
                    <RuleRow
                      mark={live.exit?.country ? <span className="text-[18px] leading-none">{flagFor(live.exit.country)}</span> : <Icon name="globe" size={18} />}
                      title={live.exit?.country ? `IP خروجی · ${countryFa(live.exit.country)}` : "IP خروجی"}
                      end={
                        live.exit ? (
                          <span dir="ltr" className="font-num select-text">
                            {live.exit.ip}
                          </span>
                        ) : live.failed ? (
                          "پیدا نشد"
                        ) : (
                          "در حال بررسی…"
                        )
                      }
                    />
                  )}
                  {on && live.conns !== null && <RuleRow mark={<Icon name="hub" size={18} />} title="اتصالات فعال" end={faDigits(live.conns)} />}
                  <RuleRow mark={<Icon name="clock" size={18} />} title="زمان باقی‌مانده" end={left === null ? "—" : `${faDigits(left)} روز`} />
                </div>
              </>
            )}
            {failed && (
              <div role="alert" className="flex flex-col gap-2.5 text-[13px] leading-[1.8] text-ink-2">
                <span>{state.message}</span>
                {state.blocking && (
                  <span className="flex">
                    <Button kind="white" icon="power" height={40} disabled={busy} onClick={disconnect}>
                      قطع و باز کردن اینترنت
                    </Button>
                  </span>
                )}
              </div>
            )}
            {on && state.note && <span className="text-xs leading-[1.8] text-ink-2">{state.note}</span>}
          </div>
        </div>

        {quota && pct !== null && (
          <div className="flex flex-col gap-2">
            <div className="flex justify-between gap-3 text-[13px] font-bold">
              <span className="truncate">
                {account?.name} — {faDigits(quota.usedGib.toFixed(1))} از {faDigits(quota.quotaGib ?? 0)} گیگ
              </span>
              <span dir="ltr" className="font-num text-ink-2">
                {pct}%
              </span>
            </div>
            <div className="h-[3px] overflow-hidden rounded-sm bg-white/16" aria-hidden="true">
              <div className="h-full bg-white" style={{ width: `${pct}%` }} />
            </div>
          </div>
        )}
      </div>

      <ServerPanel />
    </>
  );
}
