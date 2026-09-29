import { getVersion } from "@tauri-apps/api/app";
import { useEffect, useState } from "react";
import { Segmented } from "../design-system/controls";
import { Card, CardTitle, PageHeader } from "../design-system/layout";
import { faDigits } from "../lib/fa";
import { inTauri } from "../lib/platform";
import { applyTheme, loadTheme, type ThemeChoice } from "../lib/theme";

const THEMES = [
  { value: "light", label: "روشن" },
  { value: "dark", label: "تیره" },
  { value: "system", label: "هماهنگ با سیستم" },
] as const satisfies readonly { value: ThemeChoice; label: string }[];

export function Settings() {
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
      <PageHeader title="تنظیمات" subtitle="تنظیمات اتصال، Kill Switch و اتصال خودکار با هسته‌ی اتصال اضافه می‌شوند" />
      <div className="flex w-[380px] flex-col gap-4">
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
