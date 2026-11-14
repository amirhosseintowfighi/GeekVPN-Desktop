import { getVersion } from "@tauri-apps/api/app";
import { useCallback, useEffect, useState } from "react";
import { useNavigate } from "react-router";
import { Badge, Button, Hair, Segmented, Switch } from "../design-system/controls";
import { Group, PageHeader, Row } from "../design-system/layout";
import { errorText } from "../lib/auth";
import { faDigits } from "../lib/fa";
import { inTauri } from "../lib/platform";
import { system, type HelperStatus, type Mode, type Route } from "../lib/servers";
import { desktop } from "../lib/desktop";
import { currentOs } from "../lib/platform";
import { useTunnel } from "../lib/TunnelContext";
import { useUpdate } from "../shell/UpdateDialog";

const ROUTES = [
  { value: "smart", label: "هوشمند" },
  { value: "global", label: "سراسری" },
  { value: "direct", label: "مستقیم" },
] as const satisfies readonly { value: Route; label: string }[];

const ROUTE_HINT: Record<Route, string> = {
  smart: "هوشمند: سایت‌ها و IPهای ایران مستقیم، بقیه از VPN.",
  global: "سراسری: همه‌چیز از VPN، جز شبکه‌ی محلی.",
  direct: "مستقیم: همه‌چیز مستقیم؛ VPN وصل می‌ماند ولی استفاده نمی‌شود.",
};

const MODES: readonly { value: Mode; title: string; text: string }[] = [
  { value: "proxy", title: "پروکسی سیستم", text: "مرورگرها و برنامه‌هایی که پروکسی سیستم را می‌خوانند؛ بدون دسترسی مدیر" },
  { value: "tun", title: "TUN (کل سیستم)", text: "همه‌ی برنامه‌ها، بازی‌ها و ترمینال؛ لازمه‌ی Kill Switch و تونل برنامه‌ای" },
];

/** The helper that TUN mode and the kill switch need, and installing it. */
function HelperCard() {
  const [status, setStatus] = useState<HelperStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const check = useCallback(() => {
    if (inTauri) void system.helperStatus().then(setStatus, (e) => setError(errorText(e)));
  }, []);
  useEffect(check, [check]);

  const install = async () => {
    setBusy(true);
    setError(null);
    try {
      const next = await system.helperInstall();
      setStatus(next);
      // Installed but not answering yet: say why.
      if (next.state !== "ready") {
        const msg = next.state === "outdated" ? `سرویس هنوز نسخه‌ی قدیمی‌ست (${next.version}) — دوباره امتحان کن.` : next.reason;
        setError(msg);
      }
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(false);
    }
  };

  const ready = status?.state === "ready";
  return (
    <div className="flex flex-col gap-2 border-b border-hair py-3">
      <div className="flex items-center gap-2">
        <span className="flex flex-1 flex-col gap-0.5">
          <span className="text-sm font-bold text-ink">سرویس GeekVPN</span>
          <span className="text-xs leading-[1.8] text-ink-2">
            {status === null
              ? "در حال بررسی…"
              : status.state === "ready"
                ? (() => {
                    const eng = status.singBox.trim();
                    const engLabel = eng.toLowerCase().includes("hev") || eng.toLowerCase().includes("wintun") ? eng : `sing-box ${eng}`;
                    return `نصب و روشن · نسخه‌ی ${faDigits(status.version)} · ${faDigits(engLabel)}`;
                  })()
                : status.state === "outdated"
                  ? "نسخه‌ی نصب‌شده با برنامه جور نیست؛ دوباره نصبش کن."
                  : "حالت TUN و Kill Switch به این سرویس نیاز دارند. یک بار با اجازه‌ی مدیر نصب می‌شود."}
          </span>
        </span>
        {ready ? (
          <>
            <Badge tone="ok">آماده</Badge>
            <Button kind="soft" icon="dl" height={38} disabled={busy} onClick={() => void install()}>
              {busy ? "در حال نصب…" : "نصب دوباره"}
            </Button>
          </>
        ) : (
          <Button kind="action" icon="dl" height={38} disabled={busy || status === null} onClick={() => void install()}>
            {busy ? "در حال نصب…" : status?.state === "outdated" ? "نصب دوباره" : "نصب سرویس"}
          </Button>
        )}
      </div>
      {error && (
        <span role="alert" className="whitespace-pre-line text-xs leading-[1.8] text-bad">
          {error}
        </span>
      )}
      {ready && (
        <span className="text-[11px] leading-[1.8] text-ink-2">
          نکته: در حالت TUN دستور <span dir="ltr" className="font-num">ping</span> (ICMP) از تونل رد نمی‌شود — طبیعی است و نشانه‌ی قطع بودن نیست.
          برای تستِ اتصال از <span dir="ltr" className="font-num">curl https://ifconfig.me</span> یا سایت‌های «آی‌پی من چیست» استفاده کن.
        </span>
      )}
    </div>
  );
}

export function Settings() {
  const update = useUpdate();
  const { view, set, state } = useTunnel();
  const navigate = useNavigate();
  const [version, setVersion] = useState<string | null>(null);
  const [autostart, setAutostart] = useState(false);

  useEffect(() => {
    if (inTauri) void getVersion().then(setVersion);
    void desktop.autostart().then((a) => setAutostart(a.enabled), () => {});
  }, []);

  const toggleAutostart = (v: boolean) =>
    void desktop.autostart(v).then(
      (a) => setAutostart(a.enabled),
      () => {},
    );
  const os = currentOs();
  const osName = os === "windows" ? "ویندوز" : os === "macos" ? "مک" : "سیستم";

  const mode = view?.mode ?? "proxy";
  const tun = mode === "tun";
  const connected = state.status === "on" || state.status === "connecting";

  return (
    <div className="flex min-w-0 flex-1 flex-col gap-[18px]">
      <PageHeader micro="Settings" title="تنظیمات" subtitle={connected ? "تغییرهای اتصال از اتصال بعدی اعمال می‌شوند" : "تغییرها از اتصال بعدی اعمال می‌شوند"} />
      <div className="flex min-h-0 flex-1 gap-10 overflow-y-auto pb-2 pe-1 [scrollbar-width:thin] max-[1100px]:flex-col max-[1100px]:gap-5">
        <div className="flex min-w-0 flex-1 basis-0 flex-col gap-4">
          <Group micro="Connection mode">
            <div role="radiogroup" aria-label="حالت اتصال" className="flex flex-col">
              {MODES.map((m) => {
                const on = m.value === mode;
                return (
                  <button
                    key={m.value}
                    type="button"
                    role="radio"
                    aria-checked={on}
                    onClick={() => void set({ mode: m.value })}
                    className="flex items-center gap-3 border-b border-hair py-2.5 text-start last:border-b-0 hover:bg-white/5"
                  >
                    <span className="flex min-w-0 flex-1 flex-col gap-0.5">
                      <span className="text-sm font-bold">{m.title}</span>
                      <span className="text-[11px] leading-[1.7] text-ink-2">{m.text}</span>
                    </span>
                    <span className={`box-border flex h-5 w-5 shrink-0 items-center justify-center rounded-full border-[1.5px] ${on ? "border-white" : "border-white/35"}`}>
                      {on && <span className="h-2.5 w-2.5 rounded-full bg-white" />}
                    </span>
                  </button>
                );
              })}
            </div>
            {tun && <HelperCard />}
            {tun && os === "windows" && (
              <span className="py-2.5 text-[11px] leading-[1.8] text-warn">
                در TUN ویندوز «ایران مستقیم» و قانون‌های «مستقیم» و «مسدود» اعمال می‌شوند؛ تونل برنامه‌ای (Per-App) فعلاً نه.
              </span>
            )}
          </Group>
          <div className="flex flex-col gap-2">
            <span className="micro text-[9px] tracking-[0.22em]">Route</span>
            <Segmented label="مسیر ترافیک" options={ROUTES} value={view?.route ?? "smart"} onChange={(r) => void set({ route: r })} height={44} />
            <span className="text-[11px] leading-[1.8] text-ink-2">{ROUTE_HINT[view?.route ?? "smart"]}</span>
          </div>
          <Group>
            <Row
              title="تونل تفکیکی"
              hint={
                view?.apps.mode === "bypass"
                  ? `${faDigits(view.apps.paths.length)} برنامه مستقیم`
                  : view?.apps.mode === "only"
                    ? `فقط ${faDigits(view.apps.paths.length)} برنامه از VPN`
                    : "سایت‌های ایرانی، برنامه‌ها و قانون‌های دامنه و IP"
              }
              onClick={() => navigate("/settings/split")}
            />
            <Row
              title="تنظیمات پیشرفته"
              hint={
                [view?.tuning.fragment && "Fragment", view?.tuning.mux && "Mux", view?.tuning.blockIpv6 && "بدون IPv6", `DNS ${view?.tuning.dns ?? ""}`].filter(Boolean).join(" · ")
              }
              onClick={() => navigate("/settings/advanced")}
            />
          </Group>
          <Group micro="Kill switch">
            <Row
              title="Kill Switch"
              hint={tun ? "اگر تونل افتاد، اینترنت بدون VPN بسته شود" : "فقط در حالت TUN؛ با فایروال خود سیستم‌عامل"}
              trailing={<Switch label="Kill Switch" checked={tun && Boolean(view?.killSwitch)} onChange={(v) => tun && void set({ killSwitch: v })} />}
            />
            <Row
              title="حالت سخت‌گیر"
              hint="بعد از کرش یا ری‌استارت هم باز نشود تا دوباره وصل شوی"
              trailing={
                <Switch
                  label="حالت سخت‌گیر"
                  checked={tun && Boolean(view?.killSwitch && view.strict)}
                  onChange={(v) => tun && view?.killSwitch && void set({ strict: v })}
                />
              }
            />
            <Row
              title="اجازه به شبکه‌ی محلی"
              hint="پرینتر، NAS و دستگاه‌های خانه"
              trailing={<Switch label="شبکه محلی" checked={view?.allowLan ?? true} onChange={(v) => void set({ allowLan: v })} />}
            />
          </Group>
          {tun && view?.killSwitch && (
            <span className="text-[11px] leading-[1.9] text-ink-2">
              وقتی Kill Switch روشن است و اتصال می‌افتد، اینترنت تا اتصال دوباره یا زدن «قطع» بسته می‌ماند.
              {view.strict ? " در حالت سخت‌گیر، بستن برنامه یا ری‌استارت هم آن را باز نمی‌کند." : ""}
            </span>
          )}
        </div>

        <div className="flex min-w-0 flex-1 basis-0 flex-col gap-4">
          <Group micro="Auto connect">
            <Row
              title="اجرا با روشن شدن سیستم"
              hint="کوچک‌شده در کنار ساعت شروع می‌شود"
              trailing={<Switch label={`اجرا با ${osName}`} checked={autostart} onChange={toggleAutostart} />}
            />
            <Row
              title="اتصال خودکار بعد از اجرا"
              hint="با سرور و حالت انتخاب‌شده"
              trailing={<Switch label="اتصال خودکار" checked={view?.autoConnect ?? false} onChange={(v) => void set({ autoConnect: v })} />}
            />
            <Row title="شبکه‌ها" hint="روی Wi-Fi عمومی وصل شو، در خانه قطع" onClick={() => navigate("/settings/networks")} />
          </Group>
          <Group micro="General">
            <Row
              title="بستن = رفتن به کنار ساعت"
              hint="برنامه و اتصال در tray می‌مانند"
              trailing={<Switch label="کوچک به tray" checked={view?.closeToTray ?? true} onChange={(v) => void set({ closeToTray: v })} />}
            />
            <Row
              title="هشدار تمام شدن سرویس"
              hint="۸۰٪ حجم یا ۳ روز مانده"
              trailing={<Switch label="هشدار سرویس" checked={view?.expiryAlert ?? true} onChange={(v) => void set({ expiryAlert: v })} />}
            />
            <Row
              title="میانبر اتصال و قطع"
              hint={
                <span dir="ltr" className="font-num">
                  {os === "macos" ? "⌘⇧K" : "Ctrl+Shift+K"}
                </span>
              }
              trailing={<Switch label="میانبر سراسری" checked={view?.shortcut ?? true} onChange={(v) => void set({ shortcut: v })} />}
            />
          </Group>
          {version && (
            <Group micro="App">
              <Row
                title="درباره‌ی برنامه"
                hint={`نسخه‌ی ${faDigits(version)} · لایسنس GPL-3.0`}
                trailing={
                  <Button kind={update.view?.available ? "action" : "soft"} icon="dl" height={36} onClick={update.openDialog}>
                    {update.view?.available ? `نسخه‌ی ${faDigits(update.view.available.version)}` : "بررسی به‌روزرسانی"}
                  </Button>
                }
              />
            </Group>
          )}
          <Hair />
        </div>
      </div>
    </div>
  );
}
