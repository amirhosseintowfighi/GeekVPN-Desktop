import { createHashRouter, RouterProvider } from "react-router";
import { lazy, Suspense, useEffect, Component, type ReactNode, type ErrorInfo } from "react";
import { AuthProvider, useAuth } from "./lib/AuthContext";
import { desktop, windowLabel } from "./lib/desktop";
import { Flyout } from "./pages/Flyout";
import { Login } from "./pages/Login";
import { TunnelProvider } from "./lib/TunnelContext";
import { AppShell } from "./shell/AppShell";
import { Home } from "./pages/Home";

// Lazy pages: keeps the initial bundle small; Home stays eager (first paint).
const ServersPage = lazy(() => import("./pages/ServerList").then((m) => ({ default: m.ServersPage })));
const Services = lazy(() => import("./pages/Services").then((m) => ({ default: m.Services })));
const Shop = lazy(() => import("./pages/Shop").then((m) => ({ default: m.Shop })));
const Connections = lazy(() => import("./pages/Connections").then((m) => ({ default: m.Connections })));
const Tools = lazy(() => import("./pages/Tools").then((m) => ({ default: m.Tools })));
const Support = lazy(() => import("./pages/Support").then((m) => ({ default: m.Support })));
const Report = lazy(() => import("./pages/Report").then((m) => ({ default: m.Report })));
const Referral = lazy(() => import("./pages/Referral").then((m) => ({ default: m.Referral })));
const Usage = lazy(() => import("./pages/Usage").then((m) => ({ default: m.Usage })));
const Account = lazy(() => import("./pages/Account").then((m) => ({ default: m.Account })));
const Settings = lazy(() => import("./pages/Settings").then((m) => ({ default: m.Settings })));
const Split = lazy(() => import("./pages/Split").then((m) => ({ default: m.Split })));

// Catches a render crash in one page so the shell stays alive.
class ErrorBoundary extends Component<{ children: ReactNode }, { error: string | null }> {
  state: { error: string | null } = { error: null };
  static getDerivedStateFromError(e: unknown) { return { error: e instanceof Error ? e.message : String(e) }; }
  componentDidCatch(error: Error, _info: ErrorInfo) { console.error("GeekVPN page error:", error); }
  render() {
    if (this.state.error) {
      return (
        <div className="flex flex-1 flex-col items-center justify-center gap-3 p-8 text-center">
          <span className="text-lg font-bold text-ink">مشکلی پیش آمد</span>
          <span className="max-w-[420px] text-sm leading-[1.8] text-ink-2">{this.state.error}</span>
          <button type="button" onClick={() => this.setState({ error: null })} className="rounded-xl bg-action px-4 py-2 text-sm font-bold text-on-action">تلاش دوباره</button>
        </div>
      );
    }
    return this.props.children;
  }
}

function PageFallback() {
  return <div className="flex flex-1 items-center justify-center"><span className="h-8 w-8 animate-spin rounded-full border-2 border-white/30 border-t-white" aria-label="در حال بارگذاری" /></div>;
}

// Hash routing: the app is served from tauri://localhost with no server to
// answer deep paths, so a reload on /servers must not 404.
function susp(el: ReactNode) {
  return <ErrorBoundary><Suspense fallback={<PageFallback />}>{el}</Suspense></ErrorBoundary>;
}

const router = createHashRouter([
  {
    element: <AppShell />,
    children: [
      { index: true, element: <Home /> },
      { path: "servers", element: susp(<ServersPage />) },
      { path: "services", element: susp(<Services />) },
      { path: "shop", element: susp(<Shop />) },
      { path: "connections", element: susp(<Connections />) },
      { path: "tools", element: susp(<Tools />) },
      { path: "support", element: susp(<Support />) },
      { path: "report", element: susp(<Report />) },
      { path: "referral", element: susp(<Referral />) },
      { path: "usage", element: susp(<Usage />) },
      { path: "account", element: susp(<Account />) },
      { path: "settings", element: susp(<Settings />) },
      { path: "settings/split", element: susp(<Split />) },
    ],
  },
]);

/** Sign-in first, unless signed in or the customer chose to go on as a guest. */
function Gate() {
  const { view, isGuest } = useAuth();
  // «تنظیمات» and the like from the tray's panel open a page here.
  useEffect(() => {
    const off = desktop.onNavigate((path) => void router.navigate(path));
    return () => void off.then((f) => f());
  }, []);
  if (view === null) return <div className="h-full w-full bg-bg" />;
  if (!view.user && !isGuest) return <Login />;
  return (
    <TunnelProvider>
      <RouterProvider router={router} />
    </TunnelProvider>
  );
}

export function App() {
  // The small panel beside the tray icon is its own window, without the
  // shell and without the sign-in screen (that is the main window's job).
  if (windowLabel() === "flyout") {
    return (
      <AuthProvider>
        <TunnelProvider>
          <Flyout />
        </TunnelProvider>
      </AuthProvider>
    );
  }
  return (
    <AuthProvider>
      <Gate />
    </AuthProvider>
  );
}
