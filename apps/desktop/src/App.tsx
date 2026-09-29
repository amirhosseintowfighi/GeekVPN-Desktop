import { createHashRouter, RouterProvider } from "react-router";
import { AuthProvider, useAuth } from "./lib/AuthContext";
import { Account } from "./pages/Account";
import { Login } from "./pages/Login";
import { AppShell } from "./shell/AppShell";
import { Home } from "./pages/Home";
import { Section } from "./pages/Section";
import { Settings } from "./pages/Settings";

const GUEST = "برای دیدن این بخش باید وارد حسابت شوی.";

// Hash routing: the app is served from tauri://localhost with no server to
// answer deep paths, so a reload on /servers must not 404.
const router = createHashRouter([
  {
    element: <AppShell />,
    children: [
      { index: true, element: <Home /> },
      {
        path: "servers",
        element: (
          <Section
            title="سرورها"
            icon="globe"
            emptyTitle="هنوز سروری نداری"
            emptyText="سرورهای سرویس‌هایت و لینک‌های دستی اینجا فهرست می‌شوند، با تست تأخیر و ستاره."
          />
        ),
      },
      {
        path: "services",
        element: <Section title="سرویس‌های من" icon="shield" emptyTitle="سرویسی نداری" emptyText={GUEST} />,
      },
      { path: "shop", element: <Section title="فروشگاه" icon="bag" emptyTitle="فروشگاه" emptyText={GUEST} /> },
      {
        path: "connections",
        element: (
          <Section
            title="اتصالات"
            icon="hub"
            emptyTitle="اتصالی باز نیست"
            emptyText="وقتی وصل باشی، هر اتصال با مقصد، برنامه، حجم و مسیرش اینجا دیده می‌شود."
          />
        ),
      },
      {
        path: "tools",
        element: (
          <Section
            title="ابزارها"
            icon="tool"
            emptyTitle="اسکنر IP تمیز و تست سرعت"
            emptyText="اسکنر روی سرویس‌های مستقیم پشت کلادفلر کار می‌کند؛ اول یک سرویس اضافه کن."
          />
        ),
      },
      { path: "support", element: <Section title="پشتیبانی" icon="chat" emptyTitle="تیکت‌های من" emptyText={GUEST} /> },
      { path: "account", element: <Account /> },
      { path: "settings", element: <Settings /> },
    ],
  },
]);

/** Sign-in first, unless signed in or the customer chose to go on as a guest. */
function Gate() {
  const { view, isGuest } = useAuth();
  if (view === null) return <div className="h-full w-full bg-bg" />;
  if (!view.user && !isGuest) return <Login />;
  return <RouterProvider router={router} />;
}

export function App() {
  return (
    <AuthProvider>
      <Gate />
    </AuthProvider>
  );
}
