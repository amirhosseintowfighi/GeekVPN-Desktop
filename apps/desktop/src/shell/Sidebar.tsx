import { NavLink } from "react-router";
import { Icon } from "../design-system/Icon";
import { Logo } from "../design-system/Logo";
import { NAV } from "../lib/nav";

/** The right-hand rail (Desktop-Home). */
export function Sidebar() {
  return (
    <aside
      aria-label="ناوبری اصلی"
      className="glass-clear absolute bottom-3.5 right-3.5 top-3.5 z-10 flex w-[92px] flex-col items-center gap-1.5 overflow-y-auto rounded-[26px] py-3"
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
                  className={`flex items-center justify-center transition-all ${
                    isActive
                      ? "glass-milk h-11 w-11 rounded-[14px]"
                      : "h-10 w-10 rounded-[13px] bg-white/8 text-white hover:bg-white/15"
                  }`}
                >
                  <Icon name={item.icon} size={21} />
                </span>
                <span className={`text-[10px] text-white ${isActive ? "font-extrabold" : "font-medium opacity-80"}`}>{item.label}</span>
              </>
            )}
          </NavLink>
      ))}
    </aside>
  );
}
