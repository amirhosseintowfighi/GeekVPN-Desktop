import { createHashRouter, RouterProvider } from "react-router";
import { useEffect } from "react";
import { AuthProvider, useAuth } from "./lib/AuthContext";
import { desktop, windowLabel } from "./lib/desktop";
import { Flyout } from "./pages/Flyout";
import { Account } from "./pages/Account";
import { Connections } from "./pages/Connections";
import { Login } from "./pages/Login";
import { ServersPage } from "./pages/ServerList";
import { Services } from "./pages/Services";
import { TunnelProvider } from "./lib/TunnelContext";
import { AppShell } from "./shell/AppShell";
import { Home } from "./pages/Home";
import { Referral } from "./pages/Referral";
import { Report } from "./pages/Report";
import { Settings } from "./pages/Settings";
import { Shop } from "./pages/Shop";
import { Split } from "./pages/Split";
import { Support } from "./pages/Support";
import { Tools } from "./pages/Tools";
import { Usage } from "./pages/Usage";

// Hash routing: the app is served from tauri://localhost with no server to
// answer deep paths, so a reload on /servers must not 404.
const router = createHashRouter([
  {
    element: <AppShell />,
    children: [
      { index: true, element: <Home /> },
      { path: "servers", element: <ServersPage /> },
      { path: "services", element: <Services /> },
      { path: "shop", element: <Shop /> },
      { path: "connections", element: <Connections /> },
      { path: "tools", element: <Tools /> },
      { path: "support", element: <Support /> },
      { path: "report", element: <Report /> },
      { path: "referral", element: <Referral /> },
      { path: "usage", element: <Usage /> },
      { path: "account", element: <Account /> },
      { path: "settings", element: <Settings /> },
      { path: "settings/split", element: <Split /> },
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
