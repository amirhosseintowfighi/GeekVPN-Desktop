import { useCallback, useEffect, useMemo, useState } from "react";
import { Link, useNavigate, useSearchParams } from "react-router";
import { Badge, Button, Segmented } from "../design-system/controls";
import { Icon, type IconName } from "../design-system/Icon";
import { Card, CardTitle, EmptyState, PageHeader } from "../design-system/layout";
import { durationLabel, planGrid, shop, TIER_LABEL, toman, type CouponPreview, type PaymentStart, type Quote, type ShopView } from "../lib/account";
import { errorText } from "../lib/auth";
import { useAuth } from "../lib/AuthContext";
import { faDigits } from "../lib/fa";
import { useTunnel } from "../lib/TunnelContext";
import { field, PaymentResult, PendingList } from "./Payment";

const TIER_HINT: Record<string, string> = {
  direct: "مستقیم برای اسکنر کلادفلر و بیشترین سرعت",
  tunnel: "تونل برای شبکه‌های سخت‌گیر",
  elite: "ویژه با سرورهای اختصاصی",
};

/** What the bank's return page said, by `geekvpn://payment/result`. */
const PAYMENT_RESULT: Record<string, { text: string; tone: string }> = {
  ok: { text: "پرداخت تأیید شد. سرویس در «سرویس‌ها» است.", tone: "bg-ok-soft text-ok" },
  pending: { text: "پرداخت ثبت شد و در حال بررسی است. نتیجه را در ربات تلگرام هم خبر می‌دهیم.", tone: "bg-warn-soft text-warn" },
  failed: { text: "پرداخت انجام نشد. اگر مبلغ از حسابت کم شده، به پشتیبانی پیام بده.", tone: "bg-bad-soft text-bad" },
  unknown: { text: "نتیجه‌ی پرداخت مشخص نشد. اگر مبلغ کم شده، به پشتیبانی پیام بده تا پیگیری شود.", tone: "bg-warn-soft text-warn" },
};

const METHOD_ICON: Record<string, IconName> = { card: "copy", crypto: "key" };

export function WalletChip({ balance }: { balance: number | null }) {
  return (
    <Link to="/account" className="glass-clear flex h-10 items-center gap-2 rounded-[13px] pe-3 ps-1.5 text-[13px] font-bold text-white no-underline">
      <span className="flex h-7 w-7 items-center justify-center rounded-[9px] bg-white text-[#062845]">
        <Icon name="wallet" size={16} />
      </span>
      {balance === null ? "کیف پول" : `${toman(balance)} تومان`}
    </Link>
  );
}

/** Desktop-Shop: pick a plan, price it on the server, pay. */
export function Shop() {
  const { view: auth, showSignIn } = useAuth();
  const { refresh, view: servers } = useTunnel();
  const navigate = useNavigate();
  const [params] = useSearchParams();
  const renews = params.get("renew");
  const paid = params.get("payment");
  const [data, setData] = useState<ShopView | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [tier, setTier] = useState<string | null>(null);
  const [days, setDays] = useState<number | null>(null);
  const [planId, setPlanId] = useState<string | null>(null);
  const [couponText, setCouponText] = useState("");
  const [coupon, setCoupon] = useState<CouponPreview | null>(null);
  const [quote, setQuote] = useState<Quote | null>(null);
  const [method, setMethod] = useState("wallet");
  const [methodChosen, setMethodChosen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [started, setStarted] = useState<PaymentStart | null>(null);
  const [trialNote, setTrialNote] = useState<string | null>(null);

  const load = useCallback(() => {
    setLoadError(null);
    shop.load().then(setData, (e) => setLoadError(errorText(e)));
  }, []);
  useEffect(() => {
    if (auth?.user) load();
  }, [auth?.user, load, paid]);
  // Back from the bank: its page closes the payment panel, and a paid
  // service is fetched at once.
  useEffect(() => {
    if (!paid) return;
    setStarted(null);
    if (paid === "ok") void refresh();
    // Once per return; `refresh` is a new function on every render.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [paid]);

  const grid = useMemo(() => planGrid(data?.store ?? null), [data]);
  // The service being renewed decides the tier; otherwise the first on offer.
  const renewing = servers?.sources.find((s) => s.kind === "account" && s.subscriptionId === renews);
  useEffect(() => {
    if (!grid.tiers.length || (tier && grid.tiers.includes(tier))) return;
    const wanted = renewing?.kind === "account" ? renewing.tier : null;
    setTier(wanted && grid.tiers.includes(wanted) ? wanted : grid.tiers[0]!);
  }, [grid, tier, renewing]);
  const durations = tier ? grid.durations(tier) : [];
  useEffect(() => {
    if (tier && (days === null || !durations.includes(days))) setDays(durations[0] ?? null);
  }, [tier, durations, days]);
  const volumes = tier && days !== null ? grid.volumes(tier, days) : [];
  useEffect(() => {
    if (!volumes.some((p) => p.planId === planId)) setPlanId((volumes.find((p) => p.isFeatured) ?? volumes[0])?.planId ?? null);
  }, [volumes, planId]);
  const plan = volumes.find((p) => p.planId === planId) ?? null;

  const couponCode = coupon?.isValid ? coupon.code : null;
  useEffect(() => {
    setQuote(null);
    if (!planId) return;
    let live = true;
    shop.quote({ planId, coupon: couponCode, renews }).then(
      (q) => live && setQuote(q),
      (e) => live && setError(errorText(e)),
    );
    return () => {
      live = false;
    };
  }, [planId, couponCode, renews]);
  // The wallet first, unless it cannot cover this price and the customer
  // has not picked a method themselves.
  useEffect(() => {
    if (methodChosen || !quote || !data) return;
    const short = data.store.walletBalance < quote.total;
    setMethod(short ? (data.methods[0]?.key ?? "wallet") : "wallet");
  }, [quote, data, methodChosen]);
  // A code checked for one plan says nothing about another.
  useEffect(() => setCoupon(null), [planId]);

  if (!auth?.user) {
    return (
      <div className="flex min-w-0 flex-1 flex-col gap-3.5">
        <PageHeader title="فروشگاه" />
        <EmptyState
          icon="bag"
          title="برای خرید وارد شو"
          text="خرید و تمدید سرویس به حسابت در ربات تلگرام وصل است. بعد از ورود، سرویس‌ها خودشان به برنامه می‌آیند."
          action={
            <Button kind="white" icon="plane" onClick={showSignIn}>
              ورود با تلگرام
            </Button>
          }
        />
      </div>
    );
  }

  const checkCoupon = async () => {
    if (!planId || !couponText.trim()) return;
    setError(null);
    try {
      setCoupon(await shop.coupon(planId, couponText));
    } catch (e) {
      // An unknown code is refused with its reason; show it by the field.
      setCoupon({ code: couponText.trim(), isValid: false, discount: 0, totalAfter: 0, messageFa: errorText(e) });
    }
  };

  const pay = async () => {
    if (!planId) return;
    setBusy(true);
    setError(null);
    try {
      const s = await shop.checkout({ planId, coupon: couponCode, renews }, method);
      setStarted(s);
      if (s.kind === "done") void refresh();
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(false);
    }
  };

  const claimTrial = async () => {
    setBusy(true);
    setError(null);
    try {
      const r = await shop.claimTrial();
      setTrialNote(r.pending > 0 ? "تست رایگان ثبت شد؛ سرویس‌ها تا چند دقیقه‌ی دیگر می‌آیند." : "تست رایگان فعال شد. سرورهایش در «سرورها» هستند.");
      await refresh();
      load();
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(false);
    }
  };

  const finish = () => {
    setStarted(null);
    void refresh();
    load();
    if (started?.kind === "done") navigate("/services");
  };

  const balance = data?.store.walletBalance ?? null;
  const total = quote?.total ?? plan?.price ?? 0;
  const strike = quote && quote.totalDiscount > 0 ? quote.basePrice : plan?.compareAtPrice ?? null;
  const shortOfWallet = method === "wallet" && balance !== null && quote !== null && balance < quote.total;
  const methods = [{ key: "wallet", labelFa: "کیف پول" }, ...(data?.methods ?? [])];
  const summary = plan
    ? [TIER_LABEL[tier ?? ""] ?? tier, plan.quotaGib === null ? "نامحدود" : `${faDigits(plan.quotaGib)} گیگ`, `${faDigits(plan.durationDays)} روز`].join(" · ")
    : "";

  return (
    <div className="flex min-w-0 flex-1 flex-col gap-3.5">
      <PageHeader title="فروشگاه" subtitle="قیمت نهایی را فقط سرور حساب می‌کند" actions={<WalletChip balance={balance} />} />
      {renewing && (
        <div className="glass-clear flex items-center gap-2 rounded-[18px] px-4 py-2.5 text-[13px] font-bold">
          <Icon name="refresh" size={17} />
          <span className="flex-1">تمدید: {renewing.name}</span>
          <Link to="/shop" className="text-xs text-white/90">
            خرید سرویس جدید
          </Link>
        </div>
      )}
      {paid && PAYMENT_RESULT[paid] && (
        <div role="status" className={`rounded-[18px] px-4 py-3 text-[13px] font-bold ${PAYMENT_RESULT[paid]!.tone}`}>
          {PAYMENT_RESULT[paid]!.text}
        </div>
      )}
      {loadError && (
        <div role="alert" className="glass-clear flex items-center gap-3 rounded-[18px] px-4 py-3 text-[13px]">
          <span className="flex-1">{loadError}</span>
          <Button kind="white" height={36} onClick={load}>
            دوباره
          </Button>
        </div>
      )}

      <div className="flex min-h-0 flex-1 gap-4">
        <Card className="min-w-0 flex-1 basis-0 overflow-y-auto" padding={20}>
          <CardTitle title="نوع سرویس" subtitle={TIER_HINT[tier ?? ""] ?? ""} />
          {data === null && !loadError ? (
            <span className="text-[13px] text-ink-2">در حال خواندن فروشگاه…</span>
          ) : loadError ? null : grid.tiers.length === 0 ? (
            <span className="text-[13px] text-ink-2">الان سرویسی برای فروش نیست. کمی بعد سر بزن یا از پشتیبانی بپرس.</span>
          ) : (
            <>
              <Segmented
                label="نوع سرویس"
                height={46}
                options={grid.tiers.map((t) => ({ value: t, label: TIER_LABEL[t] ?? t }))}
                value={tier ?? grid.tiers[0]!}
                onChange={setTier}
              />
              <span className="mt-1 text-[13px] font-extrabold text-ink">مدت اعتبار</span>
              <div role="radiogroup" aria-label="مدت اعتبار" className="flex gap-2">
                {durations.map((d) => {
                  const on = d === days;
                  return (
                    <button
                      key={d}
                      type="button"
                      role="radio"
                      aria-checked={on}
                      onClick={() => setDays(d)}
                      className={`flex h-[66px] flex-1 basis-0 flex-col items-center justify-center gap-0.5 rounded-2xl ${
                        on ? "bg-action text-on-action" : "border border-chip-border bg-chip text-ink"
                      }`}
                    >
                      <span className="text-xl font-extrabold">{durationLabel(d).split(" ")[0]}</span>
                      <span className="text-[11px] font-semibold opacity-75">{durationLabel(d).split(" ")[1]}</span>
                    </button>
                  );
                })}
              </div>
              <div className="mt-1 flex items-center justify-between">
                <span className="text-[13px] font-extrabold text-ink">حجم (گیگابایت)</span>
                {plan?.compareAtPrice && plan.compareAtPrice > plan.price && (
                  <Badge tone="ok">٪{faDigits(Math.round((1 - plan.price / plan.compareAtPrice) * 100))} تخفیف</Badge>
                )}
              </div>
              <div role="radiogroup" aria-label="حجم" className="grid grid-cols-[repeat(auto-fill,minmax(88px,1fr))] gap-2">
                {volumes.map((p) => {
                  const on = p.planId === planId;
                  return (
                    <button
                      key={p.planId}
                      type="button"
                      role="radio"
                      aria-checked={on}
                      onClick={() => setPlanId(p.planId)}
                      className={`flex h-14 flex-col items-center justify-center rounded-[13px] text-ink ${
                        on ? "border-2 border-link bg-soft" : "border border-chip-border bg-chip"
                      }`}
                    >
                      <span className="font-num text-base font-bold">{p.quotaGib === null ? "∞" : p.quotaGib}</span>
                      <span className="text-[10px] text-ink-2">{toman(p.price)}</span>
                    </button>
                  );
                })}
              </div>
              {plan && (
                <span className="text-xs leading-[1.8] text-ink-2">
                  {plan.nameFa}
                  {plan.deviceLimit > 0 && ` · ${faDigits(plan.deviceLimit)} دستگاه هم‌زمان`}
                  {plan.dailyQuotaGib ? ` · روزانه ${faDigits(plan.dailyQuotaGib)} گیگ` : ""}
                  {plan.descriptionFa ? ` · ${plan.descriptionFa}` : ""}
                </span>
              )}
              <div className="flex items-end gap-2.5">
                <label className="flex flex-1 flex-col gap-1.5 text-xs font-bold text-ink-2">
                  کد تخفیف
                  <input
                    dir="ltr"
                    value={couponText}
                    onChange={(e) => {
                      setCouponText(e.target.value);
                      setCoupon(null);
                    }}
                    onKeyDown={(e) => e.key === "Enter" && void checkCoupon()}
                    placeholder="اگر کد داری وارد کن"
                    className={`${field} font-num`}
                  />
                </label>
                <Button kind="soft" onClick={() => void checkCoupon()} disabled={!couponText.trim() || !planId}>
                  بررسی
                </Button>
              </div>
              {coupon && (
                <span role="status" className={`text-xs ${coupon.isValid ? "text-ok" : "text-bad"}`}>
                  {coupon.messageFa || (coupon.isValid ? "کد تخفیف اعمال شد." : "این کد پذیرفته نشد.")}
                </span>
              )}
            </>
          )}
        </Card>

        <div className="flex w-[380px] shrink-0 flex-col gap-4 overflow-y-auto">
          {started ? (
            <PaymentResult start={started} amount={quote?.total ?? null} onDone={finish} onClose={() => setStarted(null)} />
          ) : (
            <>
              {data?.trial?.available && !trialNote && (
                <div className="glass-clear flex items-center gap-3 rounded-[20px] py-3 pe-3 ps-4">
                  <Icon name="gift" size={22} />
                  <span className="flex flex-1 flex-col">
                    <span className="text-sm font-extrabold">تست رایگان</span>
                    <span className="text-xs opacity-90">
                      {faDigits(data.trial.trafficMib)} مگابایت · {faDigits(data.trial.durationDays)} روز · یک بار برای هر حساب
                    </span>
                  </span>
                  <Button kind="white" height={38} disabled={busy} onClick={() => void claimTrial()}>
                    دریافت
                  </Button>
                </div>
              )}
              {trialNote && (
                <div role="status" className="glass-clear rounded-[20px] px-4 py-3 text-[13px]">
                  {trialNote}
                </div>
              )}
              <PendingList pending={data?.pending ?? []} onChanged={load} />
              <Card>
                <CardTitle title="روش پرداخت" />
                <div role="radiogroup" aria-label="روش پرداخت" className="flex flex-col gap-2">
                  {methods.map((m) => {
                    const on = m.key === method;
                    const sub =
                      m.key === "wallet"
                        ? `موجودی: ${balance === null ? "…" : toman(balance)} تومان`
                        : m.key === "card"
                          ? "بررسی رسید توسط پشتیبانی"
                          : m.key === "crypto"
                            ? "تأیید با شناسه‌ی تراکنش"
                            : "پرداخت در مرورگر";
                    return (
                      <button
                        key={m.key}
                        type="button"
                        role="radio"
                        aria-checked={on}
                        onClick={() => {
                          setMethod(m.key);
                          setMethodChosen(true);
                        }}
                        className={`flex items-center gap-3 rounded-2xl px-3.5 py-3 text-start ${
                          on ? "border-2 border-link bg-soft" : "border border-chip-border bg-chip"
                        }`}
                      >
                        <span className={`h-[18px] w-[18px] shrink-0 rounded-full border-2 ${on ? "border-[5px] border-action" : "border-check"}`} />
                        <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-[11px] bg-soft text-link">
                          <Icon name={m.key === "wallet" ? "wallet" : METHOD_ICON[m.key] ?? "globe"} size={18} />
                        </span>
                        <span className="flex flex-1 flex-col">
                          <span className="text-sm font-extrabold text-ink">{m.labelFa}</span>
                          <span className="text-xs text-ink-2">{sub}</span>
                        </span>
                      </button>
                    );
                  })}
                </div>
                {shortOfWallet && <span className="text-xs text-warn">موجودی کیف پول کافی نیست؛ روش دیگری را انتخاب کن یا کیف پول را شارژ کن.</span>}
              </Card>
              {error && (
                <div role="alert" className="rounded-2xl bg-bad-soft px-3.5 py-2.5 text-[13px] text-bad">
                  {error}
                </div>
              )}
              <section className="mt-auto flex items-center gap-3 rounded-3xl bg-pay-bar p-4 text-white shadow-[0_14px_30px_rgba(2,24,56,0.35)]">
                <span className="flex flex-1 flex-col gap-0.5">
                  {strike !== null && strike > total && <span className="text-[11px] line-through opacity-70">{toman(strike)} تومان</span>}
                  <span className="text-[19px] font-extrabold">{quote ? `${toman(quote.total)} تومان` : plan ? "در حال قیمت‌گیری…" : "—"}</span>
                  <span className="text-[11px] text-logo">{summary}</span>
                </span>
                <button
                  type="button"
                  disabled={!quote || busy || shortOfWallet}
                  onClick={() => void pay()}
                  className="flex h-[52px] items-center gap-2 rounded-[15px] bg-logo px-5 text-[15px] font-extrabold text-[#062845] disabled:opacity-50"
                >
                  <Icon name="lock" size={18} />
                  {busy ? "…" : "پرداخت"}
                </button>
              </section>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
