import { useEffect, useMemo } from "react";
import { Outlet, useNavigate } from "react-router";
import { shortcutTarget } from "../lib/nav";
import { currentOs } from "../lib/platform";
import { Backdrop } from "./Backdrop";
import { Sidebar } from "./Sidebar";
import { TitleBar } from "./TitleBar";
import { UpdateProvider } from "./UpdateDialog";

export function AppShell() {
  const os = useMemo(currentOs, []);
  const navigate = useNavigate();

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const to = shortcutTarget(e, os === "macos");
      if (to) {
        e.preventDefault();
        void navigate(to);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [navigate, os]);

  return (
    <div dir="rtl" className="relative h-full w-full overflow-hidden bg-bg text-on-bg">
      <UpdateProvider>
        <Backdrop />
        <TitleBar os={os} />
        <Sidebar />
        <main className="absolute bottom-5 left-5 right-[126px] top-[38px] flex gap-5">
          <Outlet />
        </main>
      </UpdateProvider>
    </div>
  );
}
