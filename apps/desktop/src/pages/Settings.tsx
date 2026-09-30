import { getVersion } from "@tauri-apps/api/app";
import { useEffect, useState } from "react";
import { Segmented } from "../design-system/controls";
import { Card, CardTitle, PageHeader } from "../design-system/layout";
import { faDigits } from "../lib/fa";
import { inTauri } from "../lib/platform";
import { applyTheme, loadTheme, type ThemeChoice } from "../lib/theme";
import type { Route } from "../lib/servers";
import { useTunnel } from "../lib/TunnelContext";

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
  smart: "سایت‌ها و IPهای ایران مستقیم، بقیه از VPN.",
  global: "همه‌چیز از VPN، جز شبکه‌ی محلی.",
  direct: "همه‌چیز مستقیم؛ VPN وصل می‌ماند ولی استفاده نمی‌شود.",
};

export function Settings() {
  const { view, set } = useTunnel();
  const [theme, setTheme] = useState<ThemeChoice>(loadTheme);
  const [version, setVersion] = useState<string | null>(null);

  useEffect(() => {
    if (inTauri) void getVersion().then(setVersion);
  }, []);

  const pick = (t: ThemeChoice) => {
    setTheme(t);
    applyTheme(t);
  };

  return (
    <div className="flex min-w-0 flex-1 flex-col gap-3.5">
      <PageHeader title="تنظیمات" subtitle="تغییرهای اتصال از اتصال بعدی اعمال می‌شوند" />
      <div className="flex w-[380px] flex-col gap-4">
        <Card>
          <CardTitle title="مسیر ترافیک" subtitle={ROUTE_HINT[view?.route ?? "smart"]} />
          <Segmented label="مسیر ترافیک" options={ROUTES} value={view?.route ?? "smart"} onChange={(r) => void set({ route: r })} height={42} />
        </Card>
        <Card>
          <CardTitle title="ظاهر" subtitle="هماهنگ با سیستم، تم ویندوز، مک یا لینوکس را دنبال می‌کند" />
          <Segmented label="تم برنامه" options={THEMES} value={theme} onChange={pick} height={42} />
        </Card>
        {version && (
          <Card>
            <CardTitle title="درباره‌ی برنامه" subtitle={`نسخه‌ی ${faDigits(version)} · لایسنس GPL-3.0`} />
          </Card>
        )}
      </div>
    </div>
  );
}
