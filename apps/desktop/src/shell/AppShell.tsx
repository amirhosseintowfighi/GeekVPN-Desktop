import { useEffect, useMemo } from "react";
import { Outlet, useLocation, useNavigate } from "react-router";
import { shortcutTarget } from "../lib/nav";
import { currentOs } from "../lib/platform";
import { useTunnel } from "../lib/TunnelContext";
import { Backdrop, lightFor } from "./Backdrop";
import { Sidebar } from "./Sidebar";
import { TitleBar } from "./TitleBar";
import { UpdateProvider } from "./UpdateDialog";

export function AppShell() {
  const os = useMemo(currentOs, []);
  const navigate = useNavigate();
  const { pathname } = useLocation();
  const { state } = useTunnel();
  const tunnel = state.status === "on" ? "on" : state.status === "connecting" ? "busy" : "off";

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
        <Backdrop light={lightFor(pathname, tunnel)} />
        <TitleBar os={os} />
        <div className="absolute inset-x-6 bottom-6 top-[38px] flex gap-7 isolate">
          <Sidebar />
          <main className="flex min-w-0 flex-1 gap-7 overflow-hidden py-2">
            <Outlet />
          </main>
        </div>
      </UpdateProvider>
    </div>
  );
}
