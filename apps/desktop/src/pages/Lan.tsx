import { useEffect, useState } from "react";
import { Button, Switch } from "../design-system/controls";
import { Group, PageHeader, Row } from "../design-system/layout";
import { field } from "./Payment";
import { errorText } from "../lib/auth";
import { lan, type LanPatch, type LanView } from "../lib/lan";

function Copy({ text, label }: { text: string; label: string }) {
  const [done, setDone] = useState(false);
  return (
    <button
      type="button"
      aria-label={`کپی ${label}`}
      onClick={() =>
        void navigator.clipboard.writeText(text).then(() => {
          setDone(true);
          setTimeout(() => setDone(false), 1500);
        })
      }
      className="flex items-center justify-between gap-3 border-b border-hair py-2.5 text-start last:border-b-0 hover:bg-white/5"
    >
      <span className="text-xs text-ink-2">{label}</span>
      <span dir="ltr" className="font-num truncate text-sm font-bold select-text">
        {done ? "کپی شد" : text}
      </span>
    </button>
  );
}

/** «اشتراک در شبکه‌ی محلی»: the tunnel for the phone or TV on the same Wi-Fi. */
export function Lan() {
  const [v, setV] = useState<LanView | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [port, setPort] = useState("");

  const run = (p: Promise<LanView>) =>
    p.then(
      (x) => {
        setV(x);
        setError(null);
      },
      (e) => setError(errorText(e)),
    );
  const set = (patch: LanPatch) => void run(lan.set(patch));

  useEffect(() => {
    void run(lan.view());
  }, []);
  useEffect(() => {
    if (v) setPort(String(v.settings.port));
  }, [v?.settings.port]);

  const s = v?.settings;
  const ip = v?.addresses[0];
  const portOk = /^\d+$/.test(port) && +port >= 1024 && +port <= 65535;
  const status =
    !s?.on ? null : v?.running === null ? "از اتصال بعدی روشن می‌شود." : v?.running ? "روی اتصال فعلی روشن است." : "روی اتصال فعلی روشن نیست؛ یک بار قطع و وصل کن (اگر باز روشن نشد، پورت را برنامه‌ی دیگری گرفته).";

  return (
    <div className="flex min-w-0 flex-1 flex-col gap-[18px]">
      <PageHeader micro="LAN sharing" title="اشتراک در شبکه‌ی محلی" subtitle="تنظیمات ← اشتراک · گوشی یا تلویزیون روی همین Wi-Fi از VPN این کامپیوتر استفاده کند" />
      {error && <span role="alert" className="text-[13px] text-bad">{error}</span>}
      <div className="flex min-h-0 flex-1 gap-10 overflow-y-auto pb-2 pe-1 [scrollbar-width:thin] max-[1100px]:flex-col max-[1100px]:gap-5">
        <div className="flex min-w-0 flex-1 basis-0 flex-col gap-4">
          <Group micro="Share">
            <Row
              title="اشتراک VPN در شبکه‌ی محلی"
              hint={status ?? "یک پروکسی HTTP و SOCKS5 روی همین کامپیوتر برای دستگاه‌های شبکه باز می‌شود"}
              trailing={<Switch label="اشتراک در شبکه‌ی محلی" checked={s?.on ?? false} onChange={(on) => set({ on })} />}
            />
            <Row
              title="رمز عبور"
              hint={s?.auth ? "فقط کسی که نام کاربری و رمز را دارد وصل می‌شود" : "بدون رمز هر کسی روی این شبکه می‌تواند از حجم تو استفاده کند"}
              trailing={<Switch label="رمز عبور" checked={s?.auth ?? true} onChange={(auth) => set({ auth })} />}
            />
          </Group>
          <div className="flex items-end gap-2.5">
            <label className="flex min-w-0 flex-1 flex-col gap-1.5 text-xs text-ink-2">
              پورت
              <input dir="ltr" value={port} onChange={(e) => setPort(e.target.value)} placeholder="10820" className={`${field} font-num`} />
            </label>
            <Button kind="soft" height={46} disabled={!portOk || +port === s?.port} onClick={() => set({ port: +port })}>
              ذخیره
            </Button>
          </div>
          {!portOk && <span className="text-[11px] text-bad">پورت باید عددی بین ۱۰۲۴ و ۶۵۵۳۵ باشد.</span>}
          <span className="text-[11px] leading-[1.9] text-ink-2">
            تغییرها از اتصال بعدی اعمال می‌شوند. اولین بار ویندوز یا مک ممکن است برای «geekcore» اجازه‌ی دسترسی شبکه بخواهد؛ «اجازه» را بزن. با Kill Switch روشن، «اجازه به شبکه‌ی محلی» هم باید روشن باشد.
          </span>
        </div>

        <div className="flex min-w-0 flex-1 basis-0 flex-col gap-4">
          {s?.on && !ip && <span className="text-[13px] text-ink-2">این کامپیوتر الان در شبکه‌ی محلی آدرسی ندارد.</span>}
          {s?.on && ip && (
            <>
              <div className="flex items-start gap-4">
                {v?.qr && <div className="w-[150px] shrink-0 overflow-hidden rounded-xl bg-white p-1.5 [&>svg]:h-auto [&>svg]:w-full" dangerouslySetInnerHTML={{ __html: v.qr }} />}
                <span className="text-[12px] leading-[1.9] text-ink-2">
                  در <span className="font-num">v2rayNG</span>، Shadowrocket یا NekoBox روی گوشی، «اسکن QR» را بزن. برنامه‌ی دیگری نداری؟ در تنظیمات Wi-Fi گوشی، پروکسی را «دستی» کن و آدرس و پورت را بنویس.
                </span>
              </div>
              <Group micro="Connection">
                <Copy label="آدرس" text={ip} />
                <Copy label="پورت (HTTP و SOCKS5)" text={String(s.port)} />
                {s.auth && <Copy label="نام کاربری" text={s.user} />}
                {s.auth && <Copy label="رمز" text={s.pass} />}
                {v?.link && <Copy label="لینک" text={v.link} />}
              </Group>
              {s.auth && (
                <Button kind="soft" height={38} className="self-start" onClick={() => set({ renew: true })}>
                  رمز تازه
                </Button>
              )}
              {s.auth && <span className="text-[11px] leading-[1.8] text-ink-2">تنظیم پروکسی Wi-Fi در اندروید رمز نمی‌پذیرد؛ برای آن یا از <span className="font-num">v2rayNG</span> استفاده کن یا رمز را خاموش کن.</span>}
              {v && v.addresses.length > 1 && <span className="text-[11px] text-ink-2">آدرس‌های دیگر این کامپیوتر: <span dir="ltr" className="font-num">{v.addresses.slice(1).join(", ")}</span></span>}
            </>
          )}
        </div>
      </div>
    </div>
  );
}
