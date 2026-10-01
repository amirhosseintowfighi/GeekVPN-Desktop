import { useCallback, useEffect, useState } from "react";
import { useNavigate } from "react-router";
import { Badge, Button } from "../design-system/controls";
import { Icon } from "../design-system/Icon";
import { EmptyState, Group, PageHeader, Row } from "../design-system/layout";
import { referral, shop, support, toman, wallet, type PaymentMethod, type PaymentStart, type ReferralView, type Wallet, type WalletTransactions } from "../lib/account";
import { errorText } from "../lib/auth";
import { useAuth } from "../lib/AuthContext";
import { faDigits } from "../lib/fa";
import { useUnread } from "../lib/useUnread";
import { useUpdate } from "../shell/UpdateDialog";
import { field, PaymentResult } from "./Payment";

const TX_KIND: Record<string, string> = {
  topup: "افزایش موجودی",
  purchase: "خرید",
  cashback: "بازگشت وجه",
  referral: "درآمد دعوت",
  refund: "بازپرداخت",
  adjustment: "اصلاح",
};
const PRESETS = [50_000, 100_000, 200_000, 500_000];

function Topup({ methods, onDone }: { methods: PaymentMethod[]; onDone: () => void }) {
  const [amount, setAmount] = useState("100000");
  const [method, setMethod] = useState(methods[0]?.key ?? "");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [started, setStarted] = useState<PaymentStart | null>(null);
  const value = Number(amount.replace(/[^\d]/g, ""));

  if (started) return <PaymentResult start={started} amount={value} onDone={onDone} onClose={() => setStarted(null)} />;
  const go = async () => {
    setBusy(true);
    setError(null);
    try {
      setStarted(await wallet.topup(value, method));
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <section className="glass-milk flex flex-col gap-3 rounded-3xl p-4">
      <span className="text-base font-extrabold text-ink">افزایش موجودی</span>
      <div className="grid grid-cols-4 gap-2">
        {PRESETS.map((p) => (
          <button
            key={p}
            type="button"
            onClick={() => setAmount(String(p))}
            className={`h-10 rounded-xl text-xs font-bold text-ink ${value === p ? "border-2 border-link bg-soft" : "border border-chip-border bg-chip"}`}
          >
            {toman(p)}
          </button>
        ))}
      </div>
      <label className="flex flex-col gap-1.5 text-xs font-bold text-ink-2">
        مبلغ (تومان)
        <input dir="ltr" inputMode="numeric" value={amount} onChange={(e) => setAmount(e.target.value)} className={`${field} font-num`} />
      </label>
      {methods.length === 0 ? (
        <span className="text-xs text-warn">الان روشی برای پرداخت فعال نیست.</span>
      ) : (
        <div role="radiogroup" aria-label="روش پرداخت" className="flex flex-wrap gap-2">
          {methods.map((m) => (
            <button
              key={m.key}
              type="button"
              role="radio"
              aria-checked={m.key === method}
              onClick={() => setMethod(m.key)}
              className={`h-9 rounded-xl px-3 text-xs font-bold ${m.key === method ? "bg-action text-on-action" : "border border-chip-border bg-chip text-ink"}`}
            >
              {m.labelFa}
            </button>
          ))}
        </div>
      )}
      {error && (
        <span role="alert" className="text-xs text-bad">
          {error}
        </span>
      )}
      <Button icon="lock" disabled={busy || !method || value <= 0} onClick={() => void go()}>
        {busy ? "…" : `پرداخت ${value > 0 ? toman(value) : ""} تومان`}
      </Button>
    </section>
  );
}

function Transactions() {
  const [page, setPage] = useState(1);
  const [data, setData] = useState<WalletTransactions | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    wallet.transactions(page).then(setData, (e) => setError(errorText(e)));
  }, [page]);
  const pages = data ? Math.max(1, Math.ceil(data.total / data.pageSize)) : 1;
  return (
    <section className="glass-milk flex max-h-[420px] flex-col overflow-hidden rounded-3xl">
      <div className="flex items-center gap-2 border-b border-hair px-4 py-3">
        <span className="flex-1 text-base font-extrabold text-ink">تراکنش‌ها</span>
        {pages > 1 && (
          <span className="flex items-center gap-1 text-xs text-ink-2">
            <button type="button" aria-label="صفحه‌ی قبل" disabled={page <= 1} onClick={() => setPage(page - 1)} className="p-1 disabled:opacity-40">
              <span className="flex rotate-180">
                <Icon name="chev" size={16} />
              </span>
            </button>
            {faDigits(page)} از {faDigits(pages)}
            <button type="button" aria-label="صفحه‌ی بعد" disabled={page >= pages} onClick={() => setPage(page + 1)} className="p-1 disabled:opacity-40">
              <Icon name="chev" size={16} />
            </button>
          </span>
        )}
      </div>
      <div className="overflow-y-auto">
        {error && <p className="m-0 p-4 text-[13px] text-bad">{error}</p>}
        {data?.items.length === 0 && <p className="m-0 p-6 text-center text-[13px] text-ink-2">هنوز تراکنشی نداری.</p>}
        {data?.items.map((t) => {
          const credit = t.amount > 0 && t.kind !== "purchase";
          return (
            <div key={t.transactionId} className="flex items-center gap-3 border-b border-hair px-4 py-2.5 last:border-b-0">
              <span className="flex min-w-0 flex-1 flex-col">
                <span className="text-[13px] font-bold text-ink">{t.descriptionFa || TX_KIND[t.kind] || t.kind}</span>
                <span className="text-[11px] text-muted">{new Date(t.createdAt).toLocaleDateString("fa-IR", { dateStyle: "medium" })}</span>
              </span>
              <span className={`text-[13px] font-extrabold ${credit ? "text-ok" : "text-ink"}`}>
                {credit ? "+" : "−"}
                {toman(Math.abs(t.amount))}
              </span>
            </div>
          );
        })}
      </div>
    </section>
  );
}

/** Desktop-Account: profile, wallet, referral, and the ways to support. */
export function Account() {
  const { view, signOut, showSignIn } = useAuth();
  const navigate = useNavigate();
  const unread = useUnread();
  const update = useUpdate();
  const newVersion = update.view?.available?.version ?? null;
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [money, setMoney] = useState<Wallet | null>(null);
  const [ref, setRef] = useState<ReferralView | null>(null);
  const [methods, setMethods] = useState<PaymentMethod[]>([]);
  const [panel, setPanel] = useState<"none" | "topup" | "history">("none");
  const user = view?.user;

  const load = useCallback(() => {
    wallet.load().then(setMoney, (e) => setError(errorText(e)));
    referral.load().then(setRef, () => {});
    shop.load().then((s) => setMethods(s.methods), () => {});
  }, []);
  useEffect(() => {
    if (user) load();
  }, [user, load]);

  if (!user) {
    return (
      <div className="flex min-w-0 flex-1 flex-col gap-3.5">
        <PageHeader title="حساب" />
        <EmptyState
          icon="user"
          title="وارد نشده‌ای"
          text="با ورود از طریق ربات تلگرام، سرویس‌ها و کیف پولت روی این کامپیوتر می‌آیند."
          action={
            <Button kind="white" icon="plane" onClick={showSignIn}>
              ورود با تلگرام
            </Button>
          }
        />
      </div>
    );
  }

  const leave = async () => {
    setBusy(true);
    setError(null);
    try {
      await signOut();
    } catch (e) {
      setError(errorText(e));
      setBusy(false);
    }
  };

  const r = ref?.summary;
  return (
    <div className="flex min-w-0 flex-1 flex-col gap-4">
      <div className="flex items-center gap-3.5">
        <span className="glass-clear flex h-16 w-16 items-center justify-center rounded-[21px]">
          <Icon name="user" size={30} stroke={1.8} />
        </span>
        <span className="flex flex-1 flex-col gap-1.5">
          <span className="text-[22px] font-extrabold">{user.displayName}</span>
          <span className="flex h-[26px] items-center gap-1.5 self-start rounded-[9px] bg-white/18 px-2.5 text-xs font-bold">
            <Icon name="plane" size={13} />
            متصل به تلگرام{user.username ? ` · @${user.username}` : ""} · شناسه <span dir="ltr">{faDigits(user.telegramId)}</span>
          </span>
        </span>
        <button
          type="button"
          disabled={busy}
          onClick={() => void leave()}
          className="glass-clear flex h-[42px] items-center gap-2 rounded-[13px] px-3.5 text-[13px] font-bold disabled:opacity-60"
        >
          <Icon name="logout" size={17} />
          {busy ? "در حال خروج…" : "خروج از حساب"}
        </button>
      </div>
      {error && (
        <span role="alert" className="text-[13px]">
          {error}
        </span>
      )}
      {view?.storeError && (
        <div role="status" className="flex items-start gap-2 rounded-2xl bg-warn-soft px-3.5 py-3 text-[13px] leading-[1.8] text-warn">
          <Icon name="alert" size={18} />
          <span>{view.storeError}</span>
        </div>
      )}
      <div className="flex min-h-0 flex-1 items-start gap-4 overflow-y-auto pb-2">
        <div className="flex flex-1 basis-0 flex-col gap-3.5">
          <section className="relative flex flex-col gap-3.5 overflow-hidden rounded-3xl bg-pay-bar p-[18px] text-white shadow-[0_14px_30px_rgba(2,24,56,0.35)]">
            <span className="flex items-center gap-2 text-xs opacity-80">
              <Icon name="wallet" size={16} />
              موجودی کیف پول
            </span>
            <span className="text-[32px] font-extrabold leading-none">
              {money ? toman(money.balance) : "…"} <span className="text-sm opacity-80">تومان</span>
            </span>
            {money && money.pendingCredit > 0 && <span className="text-xs opacity-80">{toman(money.pendingCredit)} تومان در راه</span>}
            <div className="flex gap-2">
              <button
                type="button"
                onClick={() => setPanel(panel === "topup" ? "none" : "topup")}
                className="flex h-[46px] flex-1 items-center justify-center gap-1.5 rounded-[14px] bg-logo text-sm font-extrabold text-[#062845]"
              >
                <Icon name="plus" size={17} />
                افزایش موجودی
              </button>
              <button
                type="button"
                onClick={() => setPanel(panel === "history" ? "none" : "history")}
                className="h-[46px] rounded-[14px] bg-white/12 px-4 text-[13px] font-bold"
              >
                تراکنش‌ها
              </button>
            </div>
          </section>
          {panel === "topup" && (
            <Topup
              methods={methods}
              onDone={() => {
                setPanel("none");
                load();
              }}
            />
          )}
          {panel === "history" && <Transactions />}
          <button type="button" onClick={() => navigate("/referral")} className="glass-clear flex items-center gap-3 rounded-[22px] px-4 py-3.5 text-start text-white">
            <Icon name="gift" size={24} />
            <span className="flex flex-1 flex-col">
              <span className="text-[15px] font-extrabold">دعوت از دوستان</span>
              <span className="text-xs opacity-90">
                {r ? `${faDigits(r.invitedCount)} دعوت · ${faDigits(r.convertedCount)} خرید · درآمد ${toman(r.totalEarned)} تومان` : "لینک دعوت و درآمدت"}
              </span>
            </span>
            <Icon name="chev" size={18} />
          </button>
          <Group label="ابزارها">
            <Row icon="chart" title="مصرف روزانه" hint="ترافیک VPN همین کامپیوتر و هر سرویس" onClick={() => navigate("/usage")} />
            <Row icon="gauge" title="تست سرعت" hint="پینگ، دانلود و آپلود" onClick={() => navigate("/tools")} />
          </Group>
        </div>
        <div className="flex flex-1 basis-0 flex-col gap-3.5">
          <Group label="پشتیبانی">
            <Row
              icon="chat"
              title="تیکت‌های من"
              hint="جواب‌های پشتیبانی، داخل برنامه"
              onClick={() => navigate("/support")}
              trailing={
                <span className="flex items-center gap-2">
                  {unread > 0 && <Badge tone="bad">{faDigits(unread)} جدید</Badge>}
                  <span className="flex text-muted">
                    <Icon name="chev" size={18} />
                  </span>
                </span>
              }
            />
            <Row icon="flag" title="گزارش مشکل" hint="یک گزارش فنی برای پشتیبانی می‌فرستد" onClick={() => navigate("/report")} />
            <Row icon="plane" title="ربات پشتیبانی" hint="گفتگو در تلگرام" onClick={() => void support.openBot().catch((e) => setError(errorText(e)))} />
          </Group>
          <Group label="برنامه">
            <Row
              icon="dl"
              title="به‌روزرسانی برنامه"
              hint={newVersion ? `نسخه‌ی ${faDigits(newVersion)} آماده است` : `نسخه‌ی ${faDigits(update.view?.current ?? "")} · بررسی نسخه‌ی جدید`}
              onClick={update.openDialog}
              trailing={
                <span className="flex items-center gap-2">
                  {newVersion && <Badge tone="link">جدید</Badge>}
                  <span className="flex text-muted">
                    <Icon name="chev" size={18} />
                  </span>
                </span>
              }
            />
          </Group>
          <Group label="حساب تلگرام">
            <Row icon="gift" title="کد معرف" hint={<span dir="ltr" className="font-num">{user.referralCode}</span>} trailing={<span />} />
          </Group>
        </div>
      </div>
    </div>
  );
}
