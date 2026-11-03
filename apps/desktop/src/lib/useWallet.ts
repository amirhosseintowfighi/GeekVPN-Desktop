import { useEffect, useState } from "react";
import { wallet } from "./account";
import { useAuth } from "./AuthContext";

const CHANGED = "geek:wallet-changed";

/** A purchase, top-up or refund happened: every balance on screen re-reads. */
export function walletChanged(): void {
  window.dispatchEvent(new Event(CHANGED));
}

/**
 * The wallet balance from `/wallet` itself (the storefront's copy can lag):
 * on mount, after `walletChanged()`, when the window comes back to the
 * front, and every minute. null until known or when signed out.
 */
export function useWalletBalance(): number | null {
  const { view } = useAuth();
  const signedIn = Boolean(view?.user);
  const [balance, setBalance] = useState<number | null>(null);
  useEffect(() => {
    if (!signedIn) {
      setBalance(null);
      return;
    }
    let alive = true;
    const read = () => wallet.load().then((w) => alive && setBalance(w.balance), () => {});
    read();
    window.addEventListener(CHANGED, read);
    window.addEventListener("focus", read);
    const t = setInterval(read, 60_000);
    return () => {
      alive = false;
      window.removeEventListener(CHANGED, read);
      window.removeEventListener("focus", read);
      clearInterval(t);
    };
  }, [signedIn]);
  return balance;
}
