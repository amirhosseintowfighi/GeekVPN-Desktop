import { getVersion } from "@tauri-apps/api/app";
import { useCallback, useEffect, useState } from "react";
import { useNavigate } from "react-router";
import { Badge, Button, Segmented, Switch } from "../design-system/controls";
import { Card, CardTitle, Group, PageHeader, Row } from "../design-system/layout";
import { errorText } from "../lib/auth";
import { faDigits } from "../lib/fa";
import { inTauri } from "../lib/platform";
import { system, type HelperStatus, type Mode, type Route } from "../lib/servers";
import { applyTheme, loadTheme, type ThemeChoice } from "../lib/theme";
import { desktop } from "../lib/desktop";
import { currentOs } from "../lib/platform";
import { useTunnel } from "../lib/TunnelContext";
import { useUpdate } from "../shell/UpdateDialog";

const THEMES = [
  { value: "light", label: "روشن" },
  { value: "dark", label: "تیره" },
  { value: "system", label: "هماهنگ با سیستم" },
] as const satisfies readonly { value: ThemeChoice; label: string }[];

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
      setStatus(await system.helperInstall());
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(false);
    }
  };

  const ready = status?.state === "ready";
  return (
    <div className="flex flex-col gap-2 rounded-2xl bg-soft px-3.5 py-3">
      <div className="flex items-center gap-2">
        <span className="flex flex-1 flex-col gap-0.5">
          <span className="text-sm font-bold text-ink">سرویس GeekVPN</span>
          <span className="text-xs leading-[1.8] text-ink-2">
            {status === null
              ? "در حال بررسی…"
              : status.state === "ready"
                ? `نصب و روشن · نسخه‌ی ${faDigits(status.version)} · sing-box ${faDigits(status.singBox)}`
                : status.state === "outdated"
                  ? "نسخه‌ی نصب‌شده با برنامه جور نیست؛ دوباره نصبش کن."
                  : "حالت TUN و Kill Switch به این سرویس نیاز دارند. یک بار با اجازه‌ی مدیر نصب می‌شود."}
          </span>
        </span>
        {ready ? (
          <Badge tone="ok">آماده</Badge>
        ) : (
          <Button kind="action" icon="dl" height={38} disabled={busy || status === null} onClick={() => void install()}>
            {busy ? "در حال نصب…" : status?.state === "outdated" ? "نصب دوباره" : "نصب سرویس"}
          </Button>
        )}
      </div>
      {error && (
        <span role="alert" className="text-xs leading-[1.8] text-bad">
          {error}
        </span>
      )}
    </div>
  );
}

export function Settings() {
  const update = useUpdate();
  const { view, set, state } = useTunnel();
  const navigate = useNavigate();
  const [theme, setTheme] = useState<ThemeChoice>(loadTheme);
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
  const osName = currentOs() === "windows" ? "ویندوز" : currentOs() === "macos" ? "مک" : "سیستم";

  const pick = (t: ThemeChoice) => {
    setTheme(t);
    applyTheme(t);
  };

  const mode = view?.mode ?? "proxy";
  const tun = mode === "tun";
  const connected = state.status === "on" || state.status === "connecting";

  return (
    <div className="flex min-w-0 flex-1 flex-col gap-3.5">
      <PageHeader title="تنظیمات" subtitle="تغییرهای اتصال از اتصال بعدی اعمال می‌شوند" />
      <div className="flex min-h-0 flex-1 gap-4 overflow-y-auto pb-2 max-[1200px]:flex-col">
        <div className="flex min-w-0 flex-1 basis-0 flex-col gap-3.5">
          <Card padding={18}>
            <CardTitle title="حالت اتصال" subtitle={connected ? "با اتصال بعدی اعمال می‌شود" : undefined} />
            <div role="radiogroup" aria-label="حالت اتصال" className="flex gap-2.5">
              {MODES.map((m) => {
                const on = m.value === mode;
                return (
                  <button
                    key={m.value}
                    type="button"
                    role="radio"
                    aria-checked={on}
                    onClick={() => void set({ mode: m.value })}
                    className={`flex flex-1 basis-0 flex-col gap-0.5 rounded-2xl p-3 text-start ${
                      on ? "border-2 border-link bg-soft" : "border border-chip-border bg-chip"
                    }`}
                  >
                    <span className="text-sm font-extrabold text-ink">{m.title}</span>
                    <span className="text-xs leading-[1.7] text-ink-2">{m.text}</span>
                  </button>
                );
              })}
            </div>
            {tun && <HelperCard />}
            <span className="text-[13px] font-extrabold text-ink">مسیر ترافیک</span>
            <Segmented label="مسیر ترافیک" options={ROUTES} value={view?.route ?? "smart"} onChange={(r) => void set({ route: r })} height={42} />
            <span className="text-xs leading-[1.8] text-ink-2">{ROUTE_HINT[view?.route ?? "smart"]}</span>
          </Card>
          <Group>
            <Row
              icon="split"
              title="تونل تفکیکی"
              hint={
                view?.apps.mode === "bypass"
                  ? `${faDigits(view.apps.paths.length)} برنامه مستقیم`
                  : view?.apps.mode === "only"
                    ? `فقط ${faDigits(view.apps.paths.length)} برنامه از VPN`
                    : "سایت‌های ایرانی و برنامه‌هایی که از VPN رد نشوند"
              }
              onClick={() => navigate("/settings/split")}
            />
          </Group>
        </div>

        <div className="flex min-w-0 flex-1 basis-0 flex-col gap-3.5">
          <Group label="Kill Switch">
            <Row
              icon="wall"
              tone="bad"
              title="Kill Switch"
              hint={tun ? "اگر تونل افتاد، اینترنت بدون VPN بسته شود" : "فقط در حالت TUN؛ با فایروال خود سیستم‌عامل"}
              trailing={<Switch label="Kill Switch" checked={tun && Boolean(view?.killSwitch)} onChange={(v) => tun && void set({ killSwitch: v })} />}
            />
            <Row
              icon="lock"
              tone="bad"
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
              icon="hub"
              title="اجازه به شبکه‌ی محلی"
              hint="پرینتر، NAS و دستگاه‌های خانه"
              trailing={<Switch label="شبکه محلی" checked={view?.allowLan ?? true} onChange={(v) => void set({ allowLan: v })} />}
            />
          </Group>
          {tun && view?.killSwitch && (
            <span className="rounded-2xl bg-white/15 px-3.5 py-2.5 text-xs leading-[1.9]">
              وقتی Kill Switch روشن است و اتصال می‌افتد، اینترنت تا اتصال دوباره یا زدن «قطع» بسته می‌ماند.
              {view.strict ? " در حالت سخت‌گیر، بستن برنامه یا ری‌استارت هم آن را باز نمی‌کند." : ""}
            </span>
          )}
        </div>

        <div className="flex w-[300px] shrink-0 flex-col gap-3.5 max-[1200px]:w-full">
          <Card>
            <CardTitle title="ظاهر" subtitle="هماهنگ با سیستم، تم ویندوز، مک یا لینوکس را دنبال می‌کند" />
            <Segmented label="تم برنامه" options={THEMES} value={theme} onChange={pick} height={42} />
          </Card>
          <Group label="عمومی">
            <Row
              icon="power"
              title="اجرا با روشن شدن سیستم"
              hint="کوچک‌شده در کنار ساعت شروع می‌شود"
              trailing={<Switch label={`اجرا با ${osName}`} checked={autostart} onChange={toggleAutostart} />}
            />
            <Row
              icon="bolt"
              title="اتصال خودکار بعد از اجرا"
              hint="با سرور و حالت انتخاب‌شده"
              trailing={<Switch label="اتصال خودکار" checked={view?.autoConnect ?? false} onChange={(v) => void set({ autoConnect: v })} />}
            />
            <Row
              icon="tray"
              title="بستن = رفتن به کنار ساعت"
              hint="برنامه و اتصال در tray می‌مانند"
              trailing={<Switch label="کوچک به tray" checked={view?.closeToTray ?? true} onChange={(v) => void set({ closeToTray: v })} />}
            />
            <Row
              icon="alert"
              title="هشدار تمام شدن سرویس"
              hint="۸۰٪ حجم یا ۳ روز مانده"
              trailing={<Switch label="هشدار سرویس" checked={view?.expiryAlert ?? true} onChange={(v) => void set({ expiryAlert: v })} />}
            />
            <Row
              icon="keyboard"
              title="میانبر اتصال و قطع"
              hint={
                <span dir="ltr" className="font-num">
                  {currentOs() === "macos" ? "⌘⇧K" : "Ctrl+Shift+K"}
                </span>
              }
              trailing={<Switch label="میانبر سراسری" checked={view?.shortcut ?? true} onChange={(v) => void set({ shortcut: v })} />}
            />
          </Group>
          {version && (
            <Card>
              <CardTitle
                title="درباره‌ی برنامه"
                subtitle={`نسخه‌ی ${faDigits(version)} · لایسنس GPL-3.0`}
                actions={
                  <Button kind={update.view?.available ? "action" : "soft"} icon="dl" height={36} onClick={update.openDialog}>
                    {update.view?.available ? `نسخه‌ی ${faDigits(update.view.available.version)}` : "بررسی به‌روزرسانی"}
                  </Button>
                }
              />
            </Card>
          )}
        </div>
      </div>
    </div>
  );
}
