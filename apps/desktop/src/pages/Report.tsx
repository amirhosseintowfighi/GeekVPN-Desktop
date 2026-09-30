import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { useEffect, useState } from "react";
import { Link } from "react-router";
import { Button } from "../design-system/controls";
import { Icon } from "../design-system/Icon";
import { Card, CardTitle, EmptyState, PageHeader } from "../design-system/layout";
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
        <PageHeader title="گزارش مشکل" />
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
    <div className="flex min-w-0 flex-1 flex-col gap-3.5">
      <PageHeader title="گزارش مشکل" subtitle="پشتیبانی ← گزارش مشکل اتصال" />
      <div className="flex min-h-0 flex-1 gap-4">
        <Card className="flex-1 basis-0" padding={20}>
          <CardTitle title="چه اتفاقی افتاد؟" subtitle="جواب را در «تیکت‌های من» و ربات تلگرام می‌گیری" />
          {sent ? (
            <div role="status" className="flex flex-col gap-3 rounded-2xl bg-ok-soft p-4 text-[13px] leading-[1.9] text-ok">
              <span>
                گزارش فرستاده شد. کد پیگیری:{" "}
                <span dir="ltr" className="font-num font-bold">
                  {sent}
                </span>
              </span>
              <Link to="/support" className="font-bold text-link">
                رفتن به تیکت‌های من
              </Link>
            </div>
          ) : (
            <>
              <textarea
                aria-label="شرح مشکل"
                value={text}
                maxLength={1500}
                onChange={(e) => setText(e.target.value)}
                placeholder="مثلاً: از صبح با حالت TUN وصل می‌شود ولی هیچ سایتی باز نمی‌شود."
                className="h-[130px] resize-none rounded-[14px] border border-chip-border bg-chip p-3 text-sm leading-[1.9] text-ink outline-none focus:border-link"
              />
              <div className="flex gap-2 rounded-[14px] bg-soft px-3 py-2.5 text-xs leading-[1.8] text-ink-2">
                <span className="flex text-link">
                  <Icon name="lock" size={18} />
                </span>
                <span>
                  گزارش شامل نسخه‌ی برنامه و سیستم‌عامل، نوع شبکه، حالت، نوع کانفیگ و آخرین خطای اتصال است. آدرس سرورها، UUID، IPها و لینک‌ها حذف می‌شوند.
                </span>
              </div>
              {error && (
                <span role="alert" className="text-xs text-bad">
                  {error}
                </span>
              )}
              <div className="flex gap-2.5">
                <Button grow icon="send" height={48} disabled={busy || text.trim().length < 10} onClick={() => void send()}>
                  {busy ? "در حال ارسال…" : "ارسال به پشتیبانی"}
                </Button>
                <Button kind="soft" icon={copied ? "check" : "copy"} height={48} disabled={!preview} onClick={() => void writeText(preview).then(() => setCopied(true))}>
                  {copied ? "کپی شد" : "کپی گزارش"}
                </Button>
              </div>
            </>
          )}
        </Card>
        <Card className="flex-1 basis-0" padding={20}>
          <CardTitle
            title="گزارش فنی"
            subtitle="همان چیزی که فرستاده می‌شود"
            actions={
              <Button kind="soft" icon="eye" height={36} onClick={() => setShown(!shown)}>
                {shown ? "پنهان کردن" : "نمایش"}
              </Button>
            }
          />
          {shown && (
            <pre dir="ltr" className="m-0 flex-1 overflow-auto whitespace-pre-wrap rounded-[14px] bg-pay-bar p-3.5 text-left font-num text-xs leading-[1.9] text-[#CFE8FF]">
              {preview || "…"}
            </pre>
          )}
        </Card>
      </div>
    </div>
  );
}
