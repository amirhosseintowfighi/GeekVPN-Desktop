import { useState } from "react";
import { Icon } from "../design-system/Icon";
import { Button } from "../design-system/controls";
import { EmptyState, Group, PageHeader, Row } from "../design-system/layout";
import { errorText } from "../lib/auth";
import { useAuth } from "../lib/AuthContext";
import { faDigits } from "../lib/fa";

/** Desktop-Account: the profile half. Wallet, referral and the rest follow in phase 5. */
export function Account() {
  const { view, signOut, showSignIn } = useAuth();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const user = view?.user;

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
            متصل به تلگرام{user.username ? ` · @${user.username}` : ""}
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
      {error && <span className="text-[13px]">{error}</span>}
      {view?.storeError && (
        <div role="status" className="flex items-start gap-2 rounded-2xl bg-warn-soft px-3.5 py-3 text-[13px] leading-[1.8] text-warn">
          <Icon name="alert" size={18} />
          <span>{view.storeError}</span>
        </div>
      )}
      <div className="w-[460px]">
        <Group label="حساب تلگرام">
          <Row icon="user" title="شناسه‌ی تلگرام" hint={<span dir="ltr">{faDigits(user.telegramId)}</span>} trailing={<span />} />
          <Row icon="gift" title="کد معرف" hint={<span dir="ltr" className="font-num">{user.referralCode}</span>} trailing={<span />} />
        </Group>
      </div>
    </div>
  );
}
