import { createContext, useContext, useEffect, useState, type ReactNode } from "react";
import { auth, guest, type AuthView } from "./auth";

interface AuthContextValue {
  /** null until the first answer from the Rust side. */
  view: AuthView | null;
  isGuest: boolean;
  continueAsGuest: () => void;
  signOut: () => Promise<void>;
  showSignIn: () => void;
}

const Ctx = createContext<AuthContextValue | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [view, setView] = useState<AuthView | null>(null);
  const [isGuest, setGuest] = useState(guest.get);

  useEffect(() => {
    let alive = true;
    void auth.state().then((v) => alive && setView(v));
    const off = auth.onChanged((v) => setView(v));
    return () => {
      alive = false;
      void off.then((f) => f());
    };
  }, []);

  const value: AuthContextValue = {
    view,
    isGuest,
    continueAsGuest: () => {
      guest.set(true);
      setGuest(true);
    },
    signOut: async () => {
      await auth.logout();
      guest.set(false);
      setGuest(false);
    },
    showSignIn: () => {
      guest.set(false);
      setGuest(false);
    },
  };
  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

export function useAuth(): AuthContextValue {
  const v = useContext(Ctx);
  if (!v) throw new Error("useAuth outside AuthProvider");
  return v;
}
