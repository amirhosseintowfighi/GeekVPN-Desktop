import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { useEffect, useState } from "react";
import { Link } from "react-router";
import { Button } from "../design-system/controls";
import { Card, EmptyState, PageHeader } from "../design-system/layout";
import { support } from "../lib/account";
import { errorText } from "../lib/auth";
import { useAuth } from "../lib/AuthContext";
import { inTauri } from "../lib/platform";

/** Desktop-Report: the customer's words and the technical report that goes with them. */
export function Report() {
  const { view, showSignIn } = useAuth();
  const [text, setText] = useState("");
  const [preview, setPreview] = useState("");
  const [shown, setShown] = useState(true);
  const [busy, setBusy] = useState(false);
  const [sent, setSent] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // What would leave, kept current as the customer writes.
  useEffect(() => {
    if (!inTauri) return;
    const t = setTimeout(() => void support.reportPreview(text).then(setPreview, () => {}), 300);
    return () => clearTimeout(t);
  }, [text]);

  if (!view?.user) {
    return (
      <div className="flex min-w-0 flex-1 flex-col gap-3.5">
        <PageHeader micro="Report" title="گزارش مشکل" />
        <EmptyState
          icon="flag"
          title="گزارش به پشتیبانی"
          text="گزارش به صورت تیکت به حسابت فرستاده می‌شود؛ برای فرستادنش وارد شو. «کپی گزارش» بدون ورود هم کار می‌کند."
          action={
            <Button kind="white" icon="plane" onClick={showSignIn}>
              ورود با تلگرام
            </Button>
          }
        />
      </div>
    );
  }

  const send = async () => {
    setBusy(true);
    setError(null);
    try {
      setSent(await support.reportSend(text));
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <>
      <div className="flex min-w-0 flex-1 flex-col gap-[18px] overflow-y-auto">
        <PageHeader micro="Report" title="گزارش مشکل" subtitle="پشتیبانی ← گزارش مشکل اتصال" />
        <span className="text-[22px] font-extrabold">چه اتفاقی افتاد؟</span>
        <span className="-mt-3 text-xs text-ink-2">جواب را در «تیکت‌های من» و ربات تلگرام می‌گیری</span>
        {sent ? (
          <div role="status" className="flex flex-col gap-2 text-sm leading-[1.9]">
            <span>
              گزارش فرستاده شد. کد پیگیری:{" "}
              <span dir="ltr" className="font-num font-bold">
                {sent}
              </span>
            </span>
            <Link to="/support" className="self-start text-[13px] font-bold text-white underline underline-offset-4">
              رفتن به تیکت‌های من
            </Link>
          </div>
        ) : (
          <>
            <label className="flex flex-col gap-2 text-xs text-ink-2">
              شرح مشکل
              <textarea
                value={text}
                maxLength={1500}
                onChange={(e) => setText(e.target.value)}
                placeholder="مثلاً: از صبح با حالت TUN وصل می‌شود ولی هیچ سایتی باز نمی‌شود."
                className="min-h-[150px] resize-none rounded-3xl border border-white/25 bg-white/7 px-[18px] py-4 text-[15px] leading-[1.8] text-ink outline-none placeholder:text-white/50 focus:border-white/60"
              />
            </label>
            <span className="text-xs leading-[1.8] text-ink-2">
              گزارش شامل نسخه‌ی برنامه و سیستم‌عامل، نوع شبکه، حالت، نوع کانفیگ و آخرین خطای اتصال است. آدرس سرورها، UUID، IPها و لینک‌ها حذف می‌شوند.
            </span>
            {error && (
              <span role="alert" className="text-xs text-bad">
                {error}
              </span>
            )}
            <div className="flex gap-2.5">
              <Button icon="send" height={40} disabled={busy || text.trim().length < 10} onClick={() => void send()}>
                {busy ? "در حال ارسال…" : "ارسال به پشتیبانی"}
              </Button>
              <Button kind="soft" icon={copied ? "check" : "copy"} height={40} disabled={!preview} onClick={() => void writeText(preview).then(() => setCopied(true))}>
                {copied ? "کپی شد" : "کپی گزارش"}
              </Button>
            </div>
          </>
        )}
      </div>
      <Card className="w-[330px] shrink-0">
        <div className="flex items-center justify-between">
          <span className="micro text-[9px]">Technical report</span>
          <button type="button" onClick={() => setShown(!shown)} className="text-xs underline underline-offset-[3px]">
            {shown ? "پنهان کردن" : "نمایش"}
          </button>
        </div>
        <span className="text-[17px] font-extrabold">گزارش فنی</span>
        <span className="text-[11px] text-ink-2">همان چیزی که فرستاده می‌شود</span>
        {shown && (
          <pre dir="ltr" className="m-0 flex-1 overflow-auto whitespace-pre-wrap rounded-[18px] border border-white/15 bg-black/35 p-3.5 text-left font-mono text-[11px] leading-[1.8] text-white/80 select-text">
            {preview || "…"}
          </pre>
        )}
      </Card>
    </>
  );
}
