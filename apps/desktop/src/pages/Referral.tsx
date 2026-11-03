import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { useEffect, useState } from "react";
import { Button, Hair, RuleRow } from "../design-system/controls";
import type { IconName } from "../design-system/Icon";
import { Card, EmptyState, PageHeader } from "../design-system/layout";
import { referral, toman, type ReferralView } from "../lib/account";
import { errorText } from "../lib/auth";
import { useAuth } from "../lib/AuthContext";
import { faDigits } from "../lib/fa";

/** Basis points as the design writes a share: «٪۱۰», «٪۲٫۵». */
export function percent(bps: number): string {
  return `٪${faDigits(Number((bps / 100).toFixed(2)))}`;
}

/** One figure of the hairline strip (Desktop-Referral). */
function Stat({ label, value, unit }: { label: string; value: string; unit: string }) {
  return (
    <div className="flex min-w-0 flex-1 basis-0 flex-col gap-1 py-3">
      <span className="micro text-[9px] tracking-[0.2em]">{label}</span>
      <span className="text-[30px] font-extrabold leading-tight">
        {value} <span className="text-[13px] font-normal text-ink-2">{unit}</span>
      </span>
    </div>
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
        <PageHeader micro="Referral" title="دعوت از دوستان" />
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
      <PageHeader micro="Referral" title="دعوت از دوستان" subtitle="حساب ← دعوت از دوستان" />
      <div className="flex min-h-0 flex-1 flex-col gap-3.5 overflow-y-auto pb-2">
      {error && (
        <span role="alert" className="text-[13px] text-bad">
          {error}
        </span>
      )}
      <Card padding={22}>
        <span className="micro text-[9px]">{data?.link ? "Your invite link" : "Your invite code"}</span>
        <div className="flex items-center gap-4">
          <div className="flex min-w-0 flex-1 flex-col gap-2.5">
            <div className="flex items-center gap-2.5">
              <span dir="ltr" className="flex h-[50px] min-w-0 flex-1 items-center truncate rounded-full border border-white/25 bg-white/7 px-4 font-num text-sm">
                {share || "…"}
              </span>
              <Button icon={copied ? "check" : "copy"} height={44} disabled={!share} onClick={() => void writeText(share).then(() => setCopied(true))}>
                {copied ? "کپی شد" : data?.link ? "کپی لینک" : "کپی کد"}
              </Button>
            </div>
            {s && (
              <span className="text-xs text-ink-2">
                کد معرف:{" "}
                <span dir="ltr" className="font-num font-bold text-white">
                  {s.code}
                </span>
              </span>
            )}
          </div>
          {data?.qrSvg && (
            // Our own QR markup, rendered by the Rust side from the link.
            <div className="rounded-[20px] bg-white p-2.5 [&>svg]:h-[112px] [&>svg]:w-[112px]" dangerouslySetInnerHTML={{ __html: data.qrSvg }} />
          )}
        </div>
      </Card>
      <div className="flex border-y border-hair">
        <Stat label="Invited" value={s ? faDigits(s.invitedCount) : "…"} unit="نفر" />
        <Hair vertical />
        <Stat label="Purchased" value={s ? faDigits(s.convertedCount) : "…"} unit="نفر" />
        <Hair vertical />
        <Stat label="Earned" value={s ? toman(s.totalEarned) : "…"} unit="تومان" />
        <Hair vertical />
        <Stat label="Pending" value={s ? toman(s.pendingEarned) : "…"} unit="تومان" />
      </div>
      <div className="flex flex-col gap-1">
        <span className="text-lg font-extrabold">شرایط</span>
        <span className="text-[11px] text-ink-2">درصدها را ادمین تعیین می‌کند؛ عددها از سرور می‌آیند</span>
        <div className="mt-2 flex flex-col border-t border-hair">
          {terms.map(([, label, value], i) => (
            <RuleRow key={label} code={String(i + 1).padStart(2, "0")} title={label} end={<span className="font-bold text-white">{value}</span>} size={14} />
          ))}
        </div>
      </div>
      </div>
    </div>
  );
}
