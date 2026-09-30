import { useEffect, useState } from "react";
import { CountryBadge, IconButton, PingBars, Segmented, Switch } from "../design-system/controls";
import { Icon } from "../design-system/Icon";
import { Logo } from "../design-system/Logo";
import { desktop } from "../lib/desktop";
import { countryCode, elapsed, plainName, speed } from "../lib/format";
import type { Route } from "../lib/servers";
import { tunnel } from "../lib/servers";
import { useTunnel } from "../lib/TunnelContext";
import { Backdrop } from "../shell/Backdrop";

const ROUTES = [
  { value: "smart", label: "هوشمند" },
  { value: "global", label: "سراسری" },
  { value: "direct", label: "مستقیم" },
] as const satisfies readonly { value: Route; label: string }[];

/**
 * The small panel beside the tray icon (Desktop-Tray): connect, the route,
 * starred servers, TUN and the kill switch, without opening the app.
 */
export function Flyout() {
  const { view, state, stats, busy, toggle, set } = useTunnel();
  const [now, setNow] = useState(Date.now());
  const on = state.status === "on";
  const connecting = state.status === "connecting";

  useEffect(() => {
    if (!on) return;
    const t = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(t);
  }, [on]);

  // Escape closes the panel, as a menu would.
  useEffect(() => {
    const k = (e: KeyboardEvent) => e.key === "Escape" && void desktop.hideSelf();
    window.addEventListener("keydown", k);
    return () => window.removeEventListener("keydown", k);
  }, []);

  const favs = (view?.servers ?? []).filter((s) => s.favorite).slice(0, 4);
  const down = speed(stats?.downBps ?? 0);
  const up = speed(stats?.upBps ?? 0);
  const tun = view?.mode === "tun";

  // Picking a starred server while connected moves the connection there.
  const pick = async (id: string) => {
    await set({ selected: id });
    if (on) void tunnel.connect().catch(() => {});
  };

  return (
    <div dir="rtl" className="relative h-screen w-screen overflow-hidden bg-bg text-on-bg">
      <Backdrop />
      <div className="absolute inset-3.5 flex flex-col gap-3">
        <div className="flex items-center gap-2.5">
          <Logo size={30} color="#FFFFFF" />
          <span dir="ltr" className="flex-1 text-right font-num text-[17px] font-bold">
            GeekVPN
          </span>
          <IconButton icon="monitor" label="باز کردن برنامه" kind="clear" size={36} onClick={() => void desktop.open()} />
          <IconButton icon="gear" label="تنظیمات" kind="clear" size={36} onClick={() => void desktop.open("/settings")} />
        </div>

        <div className="flex items-center gap-4 px-0.5 py-1">
          <button
            type="button"
            aria-label={on ? "قطع اتصال" : connecting ? "لغو اتصال" : "اتصال"}
            disabled={busy && !connecting}
            onClick={toggle}
            className={`flex h-[112px] w-[112px] shrink-0 items-center justify-center rounded-full ${
              on ? "glass-milk shadow-[0_0_0_4px_#00ACFE]" : "glass-clear"
            }`}
          >
            <span className={connecting ? "opacity-70" : ""}>
              <Logo size={58} color={on ? "#00ACFE" : "#FFFFFF"} />
            </span>
          </button>
          <div className="flex min-w-0 flex-col gap-1.5">
            <span className="flex items-center gap-2 text-sm font-bold">
              <span
                className={`h-2 w-2 rounded-[2px] ${on ? "bg-ok-bright" : connecting ? "bg-[#FFB547]" : state.status === "failed" ? "bg-bad" : "bg-white/60"}`}
              />
              {on ? "متصل و امن" : connecting ? "در حال اتصال" : state.status === "failed" ? "وصل نشد" : "قطع"}
            </span>
            {on ? (
              <>
                <span dir="ltr" className="text-right font-num text-[30px] font-bold leading-none">
                  {elapsed(state.sinceMs, now)}
                </span>
                <span dir="ltr" className="text-right font-num text-xs opacity-90">
                  ↓ {down.value} {down.unit} · ↑ {up.value} {up.unit}
                </span>
              </>
            ) : (
              <span className="line-clamp-3 text-xs leading-[1.8] opacity-90">
                {state.status === "failed" ? state.message : "برای اتصال روی عینک بزن"}
              </span>
            )}
          </div>
        </div>

        <section className="glass-milk flex flex-col gap-2.5 rounded-[20px] p-3 text-ink">
          <span className="text-xs font-extrabold text-ink-2">مسیر ترافیک</span>
          <Segmented label="مسیر ترافیک" options={ROUTES} value={view?.route ?? "smart"} onChange={(r) => void set({ route: r })} height={38} />
          <div className="flex items-center gap-2">
            <span className="flex-1 text-xs font-extrabold text-ink-2">سرورهای ستاره‌دار</span>
            <span className="text-[11px] text-ink-2">خودکار</span>
            <Switch label="انتخاب خودکار" checked={view?.autoSelect ?? true} onChange={(v) => void set({ autoSelect: v })} />
          </div>
          {favs.length === 0 ? (
            <span className="text-xs leading-[1.8] text-ink-2">در صفحه‌ی سرورها روی ستاره بزن تا اینجا بیاید.</span>
          ) : (
            favs.map((s) => {
              const selected = !view?.autoSelect && view?.selected === s.id;
              return (
                <button
                  key={s.id}
                  type="button"
                  aria-pressed={selected}
                  onClick={() => void pick(s.id)}
                  className={`flex items-center gap-2.5 rounded-[13px] px-2 py-1.5 text-start ${selected ? "bg-soft" : ""}`}
                >
                  <CountryBadge code={countryCode(s.name)} selected={selected} size={32} />
                  <span className="flex-1 truncate text-[13px] font-bold">{plainName(s.name) || s.address}</span>
                  <PingBars ms={s.delayMs ?? 0} />
                </button>
              );
            })
          )}
        </section>

        <div className="flex gap-2">
          <button
            type="button"
            aria-pressed={tun}
            onClick={() => void set({ mode: tun ? "proxy" : "tun" })}
            className={`flex h-10 flex-1 basis-0 items-center justify-center gap-2 rounded-[13px] text-[13px] font-bold ${tun ? "bg-white text-[#062845]" : "glass-clear"}`}
          >
            <Icon name="shield" size={17} />
            TUN
          </button>
          <button
            type="button"
            aria-pressed={Boolean(tun && view?.killSwitch)}
            disabled={!tun}
            title={tun ? undefined : "فقط در حالت TUN"}
            onClick={() => void set({ killSwitch: !view?.killSwitch })}
            className={`flex h-10 flex-1 basis-0 items-center justify-center gap-2 rounded-[13px] text-[13px] font-bold disabled:opacity-55 ${
              tun && view?.killSwitch ? "bg-white text-[#062845]" : "glass-clear"
            }`}
          >
            <Icon name="wall" size={17} />
            Kill Switch
          </button>
        </div>
        {(on || connecting) && <span className="text-[11px] opacity-85">تغییر حالت از اتصال بعدی اعمال می‌شود.</span>}

        <div className="mt-auto flex gap-2">
          <button type="button" onClick={() => void desktop.open()} className="glass-clear h-10 flex-1 rounded-[13px] text-[13px] font-bold">
            باز کردن GeekVPN
          </button>
          <button type="button" onClick={() => void desktop.quit()} className="h-10 rounded-[13px] bg-[rgba(217,63,72,0.9)] px-3.5 text-[13px] font-bold text-white">
            خروج
          </button>
        </div>
      </div>
    </div>
  );
}
