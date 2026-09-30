import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { open } from "@tauri-apps/plugin-dialog";
import { useState, type ReactNode } from "react";
import { Button } from "../design-system/controls";
import { Icon } from "../design-system/Icon";
import { errorText } from "../lib/auth";
import { shop, toman, type PaymentStart, type PaymentView } from "../lib/account";

export const field = "h-11 rounded-[13px] border border-chip-border bg-chip px-3 text-[13px] text-ink outline-none focus:border-link";

/** Card numbers in groups of four, left to right, as printed on the card. */
function cardGroups(n: string): string {
  return n.replace(/\D/g, "").replace(/(\d{4})(?=\d)/g, "$1 ");
}

function CopyLine({ label, value, mono = true }: { label: string; value: string; mono?: boolean }) {
  const [copied, setCopied] = useState(false);
  return (
    <div className="flex items-center gap-2 rounded-[14px] bg-soft-button px-3 py-2">
      <span className="flex min-w-0 flex-1 flex-col">
        <span className="text-[11px] text-ink-2">{label}</span>
        <span dir="ltr" className={`truncate text-right text-[15px] font-bold text-ink ${mono ? "font-num" : ""}`}>
          {value}
        </span>
      </span>
      <button
        type="button"
        onClick={() => void writeText(value.replace(/ /g, "")).then(() => setCopied(true))}
        className="flex h-9 items-center gap-1.5 rounded-xl bg-soft px-3 text-xs font-bold text-link"
      >
        <Icon name={copied ? "check" : "copy"} size={16} />
        {copied ? "کپی شد" : "کپی"}
      </button>
    </div>
  );
}

/** Picks the receipt photo and sends it; the operator reviews it in the bot. */
export function ReceiptButton({ paymentId, onSent }: { paymentId: string; onSent: () => void }) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const send = async () => {
    setError(null);
    const path = await open({ multiple: false, directory: false, title: "عکس رسید", filters: [{ name: "عکس", extensions: ["jpg", "jpeg", "png", "webp"] }] });
    if (typeof path !== "string") return;
    setBusy(true);
    try {
      await shop.receipt(paymentId, path);
      onSent();
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <div className="flex flex-col gap-1.5">
      <Button icon="file" disabled={busy} onClick={() => void send()}>
        {busy ? "در حال ارسال رسید…" : "ارسال عکس رسید"}
      </Button>
      {error && (
        <span role="alert" className="text-xs text-bad">
          {error}
        </span>
      )}
    </div>
  );
}

function Note({ tone = "link", children }: { tone?: "link" | "ok" | "warn"; children: ReactNode }) {
  const cls = tone === "ok" ? "bg-ok-soft text-ok" : tone === "warn" ? "bg-warn-soft text-warn" : "bg-soft text-ink-2";
  return <div className={`rounded-[14px] px-3 py-2.5 text-xs leading-[1.9] ${cls}`}>{children}</div>;
}

/**
 * What to do after «پرداخت»: the service is ready, a card to transfer to, a
 * bank page that opened in the browser, or a crypto address.
 */
export function PaymentResult({
  start,
  amount,
  onDone,
  onClose,
}: {
  start: PaymentStart;
  amount: number | null;
  /** The purchase or top-up is settled (or waits on review); refresh what shows it. */
  onDone: () => void;
  onClose: () => void;
}) {
  const [sent, setSent] = useState(false);
  const [txid, setTxid] = useState("");
  const [error, setError] = useState<string | null>(null);

  let body: ReactNode;
  switch (start.kind) {
    case "done":
      body = (
        <>
          <Note tone="ok">پرداخت از کیف پول انجام شد و سرویس ساخته شد. سرورهایش همین حالا در «سرورها» هستند.</Note>
          <Button onClick={onDone}>باشه</Button>
        </>
      );
      break;
    case "card": {
      const pay = start.payment;
      body = (
        <>
          <span className="text-[13px] leading-[1.9] text-ink-2">
            {toman(pay?.amount ?? amount ?? 0)} تومان را به این کارت واریز کن و بعد عکس رسید را بفرست.
          </span>
          <CopyLine label="شماره کارت" value={cardGroups(start.card.cardNumber)} />
          <div className="flex gap-2 text-xs text-ink-2">
            {start.card.cardHolderFa && <span>به نام {start.card.cardHolderFa}</span>}
            {start.card.bankFa && <span>· بانک {start.card.bankFa}</span>}
          </div>
          {pay && <CopyLine label="کد پیگیری" value={pay.reference} />}
          {sent ? (
            <Note tone="ok">رسید رسید و در صف بررسی است{start.card.reviewSlaFa ? ` (${start.card.reviewSlaFa})` : ""}. نتیجه را در ربات تلگرام هم خبر می‌دهیم.</Note>
          ) : pay ? (
            <ReceiptButton paymentId={pay.paymentId} onSent={() => setSent(true)} />
          ) : (
            <Note tone="warn">رسید را در ربات تلگرام بفرست.</Note>
          )}
          {sent && <Button onClick={onDone}>باشه</Button>}
        </>
      );
      break;
    }
    case "gateway":
      body = (
        <>
          {start.url && <Note>صفحه‌ی پرداخت در مرورگر باز شد. بعد از پرداخت به برنامه برگرد.</Note>}
          {start.bodyFa && <div className="whitespace-pre-line rounded-[14px] bg-soft-button p-3 text-[13px] leading-[1.9] text-ink">{start.bodyFa}</div>}
          <div className="flex gap-2">
            {start.url && (
              <Button kind="soft" icon="link" onClick={() => void shop.openGateway().catch((e) => setError(errorText(e)))}>
                باز کردن دوباره
              </Button>
            )}
            <Button grow icon="check" onClick={onDone}>
              پرداخت کردم
            </Button>
          </div>
        </>
      );
      break;
    case "crypto": {
      const pay = start.payment;
      const submit = async () => {
        if (!pay) return;
        setError(null);
        try {
          await shop.txid(pay.paymentId, txid);
          setSent(true);
        } catch (e) {
          setError(errorText(e));
        }
      };
      body = (
        <>
          <span className="text-[13px] leading-[1.9] text-ink-2">
            {start.amountDisplay} {start.asset} را روی شبکه‌ی {start.network} به این آدرس بفرست، بعد شناسه‌ی تراکنش را وارد کن.
          </span>
          <CopyLine label="آدرس کیف پول" value={start.address} />
          {sent ? (
            <Note tone="ok">شناسه‌ی تراکنش ثبت شد و بررسی می‌شود.</Note>
          ) : (
            pay && (
              <div className="flex gap-2">
                <input dir="ltr" value={txid} onChange={(e) => setTxid(e.target.value)} placeholder="TXID" aria-label="شناسه‌ی تراکنش" className={`${field} min-w-0 flex-1 font-num`} />
                <Button onClick={() => void submit()} disabled={txid.trim().length < 8}>
                  ثبت
                </Button>
              </div>
            )
          )}
          {sent && <Button onClick={onDone}>باشه</Button>}
        </>
      );
      break;
    }
  }

  return (
    <section aria-label="پرداخت" className="glass-milk flex flex-col gap-3 rounded-3xl p-4">
      <div className="flex items-center gap-2">
        <span className="flex h-9 w-9 items-center justify-center rounded-xl bg-soft text-link">
          <Icon name={start.kind === "done" ? "check" : "lock"} size={18} />
        </span>
        <span className="flex-1 text-base font-extrabold text-ink">
          {start.kind === "done" ? "خرید انجام شد" : start.kind === "card" ? "کارت به کارت" : start.kind === "gateway" ? "درگاه پرداخت" : "پرداخت با ارز دیجیتال"}
        </span>
        <button type="button" aria-label="بستن" onClick={onClose} className="flex h-9 w-9 items-center justify-center rounded-xl text-muted hover:bg-soft">
          <Icon name="x" size={18} />
        </button>
      </div>
      {body}
      {error && (
        <span role="alert" className="text-xs text-bad">
          {error}
        </span>
      )}
    </section>
  );
}

const PAYMENT_STATE: Record<string, string> = {
  awaiting_proof: "منتظر رسید",
  pending_review: "در حال بررسی",
};

/** Payments that still wait for a receipt or for review: one line each. */
export function PendingList({ pending, onChanged }: { pending: PaymentView[]; onChanged: () => void }) {
  const [open, setOpen] = useState<string | null>(null);
  if (!pending.length) return null;
  return (
    <section className="glass-milk flex flex-col gap-1.5 rounded-3xl p-3.5">
      <span className="px-1 text-sm font-extrabold text-ink">پرداخت‌های در جریان</span>
      {pending.map((p) => {
        const needsReceipt = p.card !== null && p.state === "awaiting_proof";
        const expanded = open === p.paymentId;
        return (
          <div key={p.paymentId} className="flex flex-col gap-2 rounded-2xl bg-soft-button px-3 py-2.5">
            <div className="flex items-center gap-2">
              <span className="flex min-w-0 flex-1 flex-col">
                <span className="whitespace-nowrap text-[13px] font-bold text-ink">{toman(p.amount)} تومان</span>
                <span className="text-[11px] text-ink-2">{PAYMENT_STATE[p.state] ?? p.state}</span>
              </span>
              {needsReceipt && (
                <button
                  type="button"
                  aria-expanded={expanded}
                  onClick={() => setOpen(expanded ? null : p.paymentId)}
                  className="h-8 whitespace-nowrap rounded-[10px] bg-soft px-3 text-xs font-bold text-link"
                >
                  {expanded ? "بستن" : "کارت و رسید"}
                </button>
              )}
            </div>
            {expanded && p.card && (
              <>
                <CopyLine label="شماره کارت" value={cardGroups(p.card.cardNumber)} />
                <ReceiptButton
                  paymentId={p.paymentId}
                  onSent={() => {
                    setOpen(null);
                    onChanged();
                  }}
                />
              </>
            )}
          </div>
        );
      })}
    </section>
  );
}
