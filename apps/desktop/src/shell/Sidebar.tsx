import { useEffect, useState } from "react";
import { NavLink, useLocation } from "react-router";
import { Icon } from "../design-system/Icon";
import { Lockup } from "../design-system/Logo";
import { toman, wallet } from "../lib/account";
import { useAuth } from "../lib/AuthContext";
import { faDigits } from "../lib/fa";
import { NAV } from "../lib/nav";
import { useUnread } from "../lib/useUnread";

/** The customer at the foot of the rail: name and wallet balance. */
function Who() {
  const { view } = useAuth();
  const user = view?.user ?? null;
  const { pathname } = useLocation();
  const [balance, setBalance] = useState<number | null>(null);
  useEffect(() => {
    if (!user) return setBalance(null);
    let alive = true;
    // Re-read when the page changes: a purchase or top-up moves it.
    wallet.load().then((w) => alive && setBalance(w.balance), () => {});
    return () => {
      alive = false;
    };
  }, [user, pathname]);
  return (
    <div className="flex flex-col gap-0.5 px-2.5 pt-3">
      <span className="truncate text-[13px] font-bold">{user ? user.displayName || `کاربر ${faDigits(user.telegramId)}` : "مهمان"}</span>
      <span className="text-xs text-ink-2">{user ? (balance === null ? "…" : `${toman(balance)} تومان`) : "وارد نشده‌ای"}</span>
    </div>
  );
}

/** The right-hand rail (Desktop-Home). */
export function Sidebar() {
  const unread = useUnread();
  return (
    <aside
      aria-label="ناوبری اصلی"
      className="glass-milk flex w-[220px] shrink-0 flex-col gap-1 overflow-y-auto rounded-[30px] px-3 py-[22px] [scrollbar-width:none]"
    >
      <div className="px-2.5 pb-4">
        <Lockup />
      </div>
      {NAV.map((item, i) => (
        <NavLink
          key={item.path}
          to={item.path}
          end={item.path === "/"}
          title={`${item.label} (Ctrl+${i + 1})`}
          className={({ isActive }) =>
            `flex h-[42px] shrink-0 items-center gap-3 rounded-[21px] px-4 text-sm no-underline transition-colors ${
              isActive ? "bg-white font-extrabold text-navy" : "font-semibold text-white/78 hover:bg-white/10"
            }`
          }
        >
          <Icon name={item.icon} size={18} />
          <span className="flex-1">{item.label}</span>
          {item.path === "/support" && unread > 0 && (
            <span
              aria-label={`${faDigits(unread)} پیام خوانده‌نشده`}
              className="flex h-[18px] min-w-[18px] items-center justify-center rounded-full bg-light px-1 text-[10px] font-extrabold text-white"
            >
              {faDigits(unread)}
            </span>
          )}
        </NavLink>
      ))}
      <div className="flex-1" />
      <div className="h-px shrink-0 bg-hair" />
      <Who />
    </aside>
  );
}
