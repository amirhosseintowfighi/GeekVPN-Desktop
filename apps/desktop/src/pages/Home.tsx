import { useState } from "react";
import { Link } from "react-router";
import { Icon, type IconName } from "../design-system/Icon";
import { Logo } from "../design-system/Logo";
import { Card, CardTitle } from "../design-system/layout";

/** One cell of the stats strip under the connect button (Desktop-Home). */
function Stat({ icon, label, value }: { icon: IconName; label: string; value: string }) {
  return (
    <div className="flex flex-1 basis-0 flex-col gap-1 px-3.5">
      <span className="flex items-center gap-1.5 text-xs opacity-85">
        <Icon name={icon} size={14} stroke={2.2} />
        {label}
      </span>
      <span className="text-[17px] font-extrabold">{value}</span>
    </div>
  );
}

/**
 * Home while there is nothing to connect with: no account and no servers yet.
 * The connecting and connected states arrive with the core (phase 3); nothing
 * here pretends to be connected.
 */
export function Home() {
  const [hint, setHint] = useState<string | null>(null);

  return (
    <>
      <div className="flex min-w-0 flex-1 basis-0 flex-col gap-4">
        <header className="flex items-center gap-3">
          <div className="flex flex-1 flex-col gap-0.5">
            <span dir="ltr" className="text-right font-num text-[22px] font-bold tracking-[-0.5px]">
              GeekVPN
            </span>
            <span className="text-[13px] opacity-90">خوش اومدی</span>
          </div>
          <Link
            to="/shop"
            className="flex h-10 items-center gap-2.5 rounded-[13px] bg-pay-bar pe-1.5 ps-3.5 text-[13px] font-bold text-white no-underline"
          >
            خرید سرویس
            <span className="flex h-7 w-7 items-center justify-center rounded-[9px] bg-white text-[#062845]">
              <Icon name="plus" size={16} stroke={2.6} />
            </span>
          </Link>
        </header>

        <div className="flex flex-1 items-center gap-9 px-3">
          <div className="relative flex h-[280px] w-[280px] shrink-0 items-center justify-center">
            <svg aria-hidden="true" width="280" height="280" viewBox="0 0 280 280" className="absolute inset-0">
              <circle cx="140" cy="140" r="128" fill="none" stroke="rgba(255,255,255,0.22)" strokeWidth="6" />
            </svg>
            <button
              type="button"
              aria-label="اتصال"
              onClick={() => setHint("اول یک سرویس بخر یا لینک اشتراکت را در «سرویس‌ها» اضافه کن.")}
              className="glass-clear relative flex h-[236px] w-[236px] items-center justify-center rounded-full shadow-[0_0_0_14px_rgba(255,255,255,0.08),0_24px_50px_rgba(2,36,84,0.3)] transition-transform active:scale-[0.98]"
            >
              <Logo size={132} color="#FFFFFF" />
            </button>
          </div>
          <div className="flex flex-1 flex-col gap-3.5">
            <span className="flex items-center gap-2 text-[15px] font-bold">
              <span className="h-[9px] w-[9px] rounded-[2px] bg-white/60" />
              قطع
            </span>
            <span className="text-[28px] font-extrabold leading-tight">هنوز سروری نداری</span>
            <span className="text-[13px] opacity-90" role="status">
              {hint ?? "برای اتصال، روی عینک بزن"}
            </span>
            <Link
              to="/services"
              className="glass-milk mt-1.5 flex max-w-[380px] items-center gap-3 rounded-[20px] py-2.5 pe-3.5 ps-2.5 no-underline"
            >
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
          </div>
        </div>

        <div className="glass-clear flex rounded-[20px] px-1 py-3.5">
          <Stat icon="dl" label="دانلود" value="—" />
          <span aria-hidden="true" className="w-px self-stretch bg-white/25" />
          <Stat icon="up" label="آپلود" value="—" />
          <span aria-hidden="true" className="w-px self-stretch bg-white/25" />
          <Stat icon="globe" label="IP خروجی" value="—" />
          <span aria-hidden="true" className="w-px self-stretch bg-white/25" />
          <Stat icon="hub" label="اتصالات فعال" value="—" />
          <span aria-hidden="true" className="w-px self-stretch bg-white/25" />
          <Stat icon="clock" label="زمان باقی‌مانده" value="—" />
        </div>
      </div>

      <Card className="w-[360px] shrink-0 rounded-[26px]">
        <CardTitle title="سرورها" subtitle="هنوز سروری اضافه نشده" />
        <div className="flex flex-1 flex-col items-center justify-center gap-3 text-center text-ink">
          <span className="flex h-16 w-16 items-center justify-center rounded-[21px] bg-soft text-link">
            <Icon name="globe" size={30} stroke={1.8} />
          </span>
          <span className="text-base font-extrabold">فهرست سرورها خالی است</span>
          <span className="max-w-[260px] text-[13px] leading-[1.9] text-ink-2">
            با ورود به حسابت سرویس‌هایت خودکار اینجا می‌آیند؛ یا لینک اشتراک خودت را اضافه کن.
          </span>
        </div>
        <Link
          to="/account"
          className="flex h-11 items-center justify-center gap-2 rounded-[14px] bg-action text-sm font-bold text-on-action no-underline shadow-[var(--gv-action-shadow)]"
        >
          <Icon name="plane" size={18} />
          ورود با تلگرام
        </Link>
      </Card>
    </>
  );
}
