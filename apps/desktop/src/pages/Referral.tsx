import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { useEffect, useState } from "react";
import { Button } from "../design-system/controls";
import { Icon, type IconName } from "../design-system/Icon";
import { Card, CardTitle, EmptyState, PageHeader } from "../design-system/layout";
import { referral, toman, type ReferralView } from "../lib/account";
import { errorText } from "../lib/auth";
import { useAuth } from "../lib/AuthContext";
import { faDigits } from "../lib/fa";

/** Basis points as the design writes a share: «٪۱۰», «٪۲٫۵». */
export function percent(bps: number): string {
  return `٪${faDigits(Number((bps / 100).toFixed(2)))}`;
}

function Stat({ label, value, unit }: { label: string; value: string; unit: string }) {
  return (
    <Card className="flex-1 basis-0">
      <span className="text-xs text-ink-2">{label}</span>
      <span className="text-[30px] font-extrabold leading-tight text-ink">
        {value} <span className="text-[13px] text-ink-2">{unit}</span>
      </span>
    </Card>
  );
}

/** Desktop-Referral: the invite link, what it earned, and the terms. */
export function Referral() {
  const { view, showSignIn } = useAuth();
  const [data, setData] = useState<ReferralView | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const signedIn = Boolean(view?.user);

  useEffect(() => {
    if (signedIn) referral.load().then(setData, (e) => setError(errorText(e)));
  }, [signedIn]);

  if (!signedIn) {
    return (
      <div className="flex min-w-0 flex-1 flex-col gap-3.5">
        <PageHeader title="دعوت از دوستان" />
        <EmptyState
          icon="gift"
          title="لینک دعوت تو"
          text="برای گرفتن لینک دعوت وارد حسابت شو."
          action={
            <Button kind="white" icon="plane" onClick={showSignIn}>
              ورود با تلگرام
            </Button>
          }
        />
      </div>
    );
  }

  const s = data?.summary;
  const share = data?.link ?? s?.code ?? "";
  const terms: [IconName, string, string][] = s
    ? [
        ["gift", "هدیه‌ی عضویت دوستت", `${toman(s.inviteeBonus)} تومان`],
        ["bag", "از اولین خرید", percent(s.firstPurchaseBps)],
        ["refresh", "از خریدهای بعدی", percent(s.recurringBps)],
      ]
    : [];

  return (
    <div className="flex min-w-0 flex-1 flex-col gap-3.5">
      <PageHeader title="دعوت از دوستان" subtitle="حساب ← دعوت از دوستان" />
      <div className="flex min-h-0 flex-1 flex-col gap-3.5 overflow-y-auto pb-2">
      {error && (
        <span role="alert" className="text-[13px]">
          {error}
        </span>
      )}
      <section className="relative flex items-center gap-6 overflow-hidden rounded-[26px] bg-pay-bar p-6 text-white shadow-[0_14px_30px_rgba(2,24,56,0.35)]">
        <div className="flex flex-1 flex-col gap-2.5">
          <span className="text-[13px] opacity-80">{data?.link ? "لینک دعوت تو" : "کد دعوت تو"}</span>
          <div className="flex items-center gap-2.5">
            <span dir="ltr" className="flex h-[50px] min-w-0 flex-1 items-center truncate rounded-[14px] bg-white/10 px-3.5 font-num text-[15px] font-semibold">
              {share || "…"}
            </span>
            <button
              type="button"
              disabled={!share}
              onClick={() => void writeText(share).then(() => setCopied(true))}
              className="flex h-[50px] items-center gap-2 rounded-[14px] bg-logo px-[18px] text-sm font-extrabold text-[#062845] disabled:opacity-50"
            >
              <Icon name={copied ? "check" : "copy"} size={18} />
              {copied ? "کپی شد" : data?.link ? "کپی لینک" : "کپی کد"}
            </button>
          </div>
          {s && (
            <span className="text-[13px] opacity-85">
              کد معرف:{" "}
              <span dir="ltr" className="font-num font-bold text-logo">
                {s.code}
              </span>
            </span>
          )}
        </div>
        {data?.qrSvg && (
          // Our own QR markup, rendered by the Rust side from the link.
          <div className="rounded-[20px] bg-white p-3 [&>svg]:h-[132px] [&>svg]:w-[132px]" dangerouslySetInnerHTML={{ __html: data.qrSvg }} />
        )}
      </section>
      <div className="flex gap-4">
        <Stat label="دعوت‌شده" value={s ? faDigits(s.invitedCount) : "…"} unit="نفر" />
        <Stat label="خرید کرده" value={s ? faDigits(s.convertedCount) : "…"} unit="نفر" />
        <Stat label="درآمد" value={s ? toman(s.totalEarned) : "…"} unit="تومان" />
        <Stat label="در انتظار" value={s ? toman(s.pendingEarned) : "…"} unit="تومان" />
      </div>
      <Card padding={18}>
        <CardTitle title="شرایط" subtitle="درصدها را ادمین تعیین می‌کند؛ عددها از سرور می‌آیند" />
        {terms.map(([icon, label, value]) => (
          <div key={label} className="flex items-center gap-3 border-b border-hair py-2.5 last:border-b-0">
            <span className="flex h-9 w-9 items-center justify-center rounded-[11px] bg-soft text-link">
              <Icon name={icon} size={18} />
            </span>
            <span className="flex-1 text-sm font-bold text-ink">{label}</span>
            <span className="text-sm font-extrabold text-ink">{value}</span>
          </div>
        ))}
      </Card>
      </div>
    </div>
  );
}
