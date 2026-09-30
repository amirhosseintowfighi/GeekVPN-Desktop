import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useMemo, useState } from "react";
import { Button, IconButton, SearchField, Segmented, Switch } from "../design-system/controls";
import { Icon } from "../design-system/Icon";
import { Card, CardTitle, PageHeader } from "../design-system/layout";
import { errorText } from "../lib/auth";
import { faDigits } from "../lib/fa";
import { inTauri } from "../lib/platform";
import { system, type AppMode, type Program } from "../lib/servers";
import { useTunnel } from "../lib/TunnelContext";

const APP_MODES = [
  { value: "off", label: "همه از VPN" },
  { value: "bypass", label: "این‌ها مستقیم" },
  { value: "only", label: "فقط این‌ها از VPN" },
] as const satisfies readonly { value: AppMode; label: string }[];

const APP_HINT: Record<AppMode, string> = {
  off: "همه‌ی برنامه‌ها از مسیر ترافیک پیروی می‌کنند.",
  bypass: "برنامه‌های این فهرست بدون VPN وصل می‌شوند؛ بقیه از VPN.",
  only: "فقط برنامه‌های این فهرست از VPN رد می‌شوند؛ بقیه مستقیم.",
};

/** The last path segment, without an .exe ending. */
function programName(path: string): string {
  const file = path.split(/[\\/]/).pop() ?? path;
  return file.replace(/\.exe$/i, "");
}

export function Split() {
  const { view, set } = useTunnel();
  const [running, setRunning] = useState<Program[] | null>(null);
  const [query, setQuery] = useState("");
  const [error, setError] = useState<string | null>(null);

  const apps = view?.apps ?? { mode: "off" as AppMode, paths: [] };
  const tun = view?.mode === "tun";

  const loadRunning = () => {
    if (!inTauri) return;
    setError(null);
    void system.programs().then(setRunning, (e) => setError(errorText(e)));
  };
  useEffect(loadRunning, []);

  const savePaths = (paths: string[]) => void set({ apps: { mode: apps.mode === "off" && paths.length ? "bypass" : apps.mode, paths } });
  const add = (path: string) => {
    if (!apps.paths.includes(path)) savePaths([...apps.paths, path]);
  };

  const pickFile = async () => {
    try {
      const chosen = await open({ multiple: false, directory: false, title: "انتخاب برنامه" });
      if (typeof chosen === "string") add(chosen);
    } catch (e) {
      setError(errorText(e));
    }
  };

  const shown = useMemo(() => {
    const q = query.trim().toLowerCase();
    return (running ?? []).filter((p) => !apps.paths.includes(p.path) && (!q || p.name.toLowerCase().includes(q) || p.path.toLowerCase().includes(q)));
  }, [running, query, apps.paths]);

  return (
    <div className="flex min-w-0 flex-1 flex-col gap-3.5">
      <PageHeader title="تونل تفکیکی" subtitle="تنظیمات ← تونل تفکیکی · تغییرها از اتصال بعدی اعمال می‌شوند" />

      <section className="flex items-center gap-4 rounded-3xl bg-[#062845] p-[18px] text-white shadow-[0_14px_30px_rgba(2,24,56,0.35)]">
        <span className="flex h-[54px] w-[54px] shrink-0 items-center justify-center rounded-[17px] bg-logo text-[#062845]">
          <Icon name="split" size={26} />
        </span>
        <span className="flex flex-1 flex-col gap-1">
          <span className="text-[17px] font-extrabold">سایت‌های ایرانی مستقیم</span>
          <span className="text-xs leading-[1.8] opacity-85">
            دامنه‌ها و IPهای ایران (geosite:category-ir و geoip:ir) از VPN رد نمی‌شوند؛ بانک‌ها و درگاه‌ها بدون خطا کار می‌کنند. همان «مسیر هوشمند» تنظیمات است.
          </span>
        </span>
        <Switch onNavy label="سایت‌های ایرانی مستقیم" checked={view?.route === "smart"} onChange={(v) => void set({ route: v ? "smart" : "global" })} />
      </section>

      {!tun && (
        <div className="glass-clear flex items-center gap-3 rounded-[20px] px-4 py-3">
          <span className="flex-1 text-[13px] leading-[1.8]">
            تونل برنامه‌ای فقط در حالت TUN کار می‌کند؛ در حالت پروکسی سیستم، هر برنامه خودش تصمیم می‌گیرد پروکسی را بخواند یا نه.
          </span>
          <Button kind="white" icon="shield" height={38} onClick={() => void set({ mode: "tun" })}>
            رفتن به حالت TUN
          </Button>
        </div>
      )}

      <div className="flex min-h-0 flex-1 gap-4">
        <section className="glass-milk flex min-w-0 flex-1 flex-col overflow-hidden rounded-[26px]">
          <div className="flex flex-col gap-3 border-b border-hair p-4">
            <CardTitle title="برنامه‌ها" subtitle="فقط در حالت TUN · با مسیر فایل اجرایی شناخته می‌شوند" actions={
              <Button kind="action" icon="folder" height={40} onClick={() => void pickFile()}>
                انتخاب فایل اجرایی
              </Button>
            } />
            <Segmented label="تونل برنامه‌ای" options={APP_MODES} value={apps.mode} onChange={(m) => void set({ apps: { ...apps, mode: m } })} height={40} />
            <span className="text-xs text-ink-2">{APP_HINT[apps.mode]}</span>
          </div>
          <div className="flex-1 overflow-y-auto">
            {apps.paths.length === 0 ? (
              <p className="m-0 p-6 text-center text-[13px] leading-[1.9] text-ink-2">
                هنوز برنامه‌ای اضافه نشده. از فهرست برنامه‌های در حال اجرا یا با «انتخاب فایل اجرایی» اضافه کن.
              </p>
            ) : (
              apps.paths.map((p) => (
                <div key={p} className="grid grid-cols-[40px_minmax(0,1fr)_36px] items-center gap-3 border-b border-hair px-4 py-2.5 last:border-b-0">
                  <span className="flex h-10 w-10 items-center justify-center rounded-xl bg-soft font-num text-sm font-bold text-link">
                    {programName(p).slice(0, 1).toUpperCase()}
                  </span>
                  <span className="flex min-w-0 flex-col">
                    <span className="text-sm font-extrabold text-ink">{programName(p)}</span>
                    <span dir="ltr" className="truncate text-right font-num text-[11px] text-muted">
                      {p}
                    </span>
                  </span>
                  <IconButton icon="trash" label={`حذف ${programName(p)}`} size={36} onClick={() => savePaths(apps.paths.filter((x) => x !== p))} />
                </div>
              ))
            )}
          </div>
        </section>

        <Card className="w-[360px] shrink-0">
          <CardTitle
            title="برنامه‌های در حال اجرا"
            subtitle={running ? `${faDigits(running.length)} برنامه` : "در حال خواندن…"}
            actions={<IconButton icon="refresh" label="تازه کردن فهرست" onClick={loadRunning} />}
          />
          <SearchField id="program-search" placeholder="جستجوی برنامه" value={query} onChange={setQuery} />
          {error && (
            <span role="alert" className="text-xs text-bad">
              {error}
            </span>
          )}
          <div className="-mx-1 flex max-h-[420px] flex-col overflow-y-auto">
            {shown.map((p) => (
              <button
                key={p.path}
                type="button"
                onClick={() => add(p.path)}
                className="flex items-center gap-2.5 rounded-xl px-2 py-2 text-start hover:bg-soft"
                title={p.path}
              >
                <span className="flex min-w-0 flex-1 flex-col">
                  <span className="truncate text-[13px] font-bold text-ink">{p.name}</span>
                  <span dir="ltr" className="truncate text-right font-num text-[11px] text-muted">
                    {p.path}
                  </span>
                </span>
                <span className="flex text-link">
                  <Icon name="plus" size={18} />
                </span>
              </button>
            ))}
          </div>
        </Card>
      </div>
    </div>
  );
}
