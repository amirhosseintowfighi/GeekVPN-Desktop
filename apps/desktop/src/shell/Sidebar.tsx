import { NavLink } from "react-router";
import { Icon } from "../design-system/Icon";
import { Logo } from "../design-system/Logo";
import { NAV } from "../lib/nav";
import { faDigits } from "../lib/fa";
import { useUnread } from "../lib/useUnread";

/** The right-hand rail (Desktop-Home). */
export function Sidebar() {
  const unread = useUnread();
  return (
    <aside
      aria-label="ناوبری اصلی"
      className="glass-clear absolute bottom-3.5 right-3.5 top-3.5 z-10 flex w-[92px] flex-col items-center gap-1.5 overflow-y-auto rounded-[26px] py-3 [scrollbar-width:thin] [scrollbar-color:rgba(255,255,255,0.25)_transparent]"
    >
      <span className="mb-1.5">
        <Logo size={40} color="#FFFFFF" title="GeekVPN" />
      </span>
      {NAV.map((item, i) => (
          <NavLink
            key={item.path}
            to={item.path}
            end={item.path === "/"}
            title={`${item.label} (Ctrl+${i + 1})`}
            className="flex w-[76px] flex-col items-center gap-1 no-underline"
          >
            {({ isActive }) => (
              <>
                <span
                  className={`relative flex items-center justify-center transition-all ${
                    isActive
                      ? "glass-milk h-11 w-11 rounded-[14px]"
                      : "h-10 w-10 rounded-[13px] bg-white/8 text-white hover:bg-white/15"
                  }`}
                >
                  <Icon name={item.icon} size={21} />
                  {item.path === "/support" && unread > 0 && (
                    <span
                      aria-label={`${faDigits(unread)} پیام خوانده‌نشده`}
                      className="absolute -left-1.5 -top-1.5 flex h-[18px] min-w-[18px] items-center justify-center rounded-full bg-bad px-1 text-[10px] font-extrabold text-white"
                    >
                      {faDigits(unread)}
                    </span>
                  )}
                </span>
                <span className={`text-[10px] text-white ${isActive ? "font-extrabold" : "font-medium opacity-80"}`}>{item.label}</span>
              </>
            )}
          </NavLink>
      ))}
    </aside>
  );
}
