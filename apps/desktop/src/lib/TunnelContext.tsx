import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { errorText } from "./auth";
import { useAuth } from "./AuthContext";
import { windowLabel } from "./desktop";
import { servers, tunnel, type ServerSettings, type ServersView, type TunnelState, type TunnelStats } from "./servers";

/** Seconds of speed history the live graph keeps. */
export const HISTORY = 60;

interface TunnelContextValue {
  view: ServersView | null;
  state: TunnelState;
  stats: TunnelStats | null;
  history: TunnelStats[];
  testing: boolean;
  busy: boolean;
  error: string | null;
  clearError: () => void;
  toggle: () => void;
  /** «قطع» from any state, including a failure the kill switch holds shut. */
  disconnect: () => void;
  refresh: () => Promise<void>;
  test: () => Promise<void>;
  add: (text: string) => Promise<boolean>;
  removeSource: (id: string) => Promise<void>;
  set: (s: ServerSettings) => Promise<void>;
}

const Ctx = createContext<TunnelContextValue | null>(null);

export function TunnelProvider({ children }: { children: ReactNode }) {
  const [view, setView] = useState<ServersView | null>(null);
  const [state, setState] = useState<TunnelState>({ status: "off" });
  const [history, setHistory] = useState<TunnelStats[]>([]);
  const [testing, setTesting] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const { view: auth } = useAuth();

  const reload = useCallback(() => servers.state().then(setView), []);

  useEffect(() => {
    void reload();
    void tunnel.state().then(setState);
    const offs = [
      servers.onChanged(() => void reload()),
      servers.onDelay((r) =>
        setView((v) => v && { ...v, servers: v.servers.map((s) => (s.id === r.id ? { ...s, delayMs: r.ms } : s)) }),
      ),
      tunnel.onState((s) => {
        setState(s);
        if (s.status !== "on") setHistory([]);
      }),
      tunnel.onStats((st) => setHistory((h) => [...h.slice(-(HISTORY - 1)), st])),
    ];
    return () => offs.forEach((p) => void p.then((f) => f()));
  }, [reload]);

  // Signing in (or out) changes which services the account has.
  const userId = auth?.user?.id ?? null;
  useEffect(() => {
    // The main window keeps the account in sync; the tray panel only reads.
    if (userId && windowLabel() === "main") void servers.refresh().then(setView).catch((e) => setError(errorText(e)));
  }, [userId]);

  const guard = async <T,>(f: () => Promise<T>): Promise<T | undefined> => {
    setError(null);
    try {
      return await f();
    } catch (e) {
      setError(errorText(e));
      return undefined;
    }
  };

  const memoizedValue: TunnelContextValue = useMemo(() => ({
    view,
    state,
    stats: history.at(-1) ?? null,
    history,
    testing,
    busy,
    error,
    clearError: () => setError(null),
    toggle: () => {
      if (busy) return;
      setBusy(true);
      // A failed connect already arrives as a "failed" state with its message.
      const act = state.status === "on" || state.status === "connecting" ? tunnel.disconnect() : tunnel.connect();
      void act.catch(() => {}).finally(() => setBusy(false));
    },
    disconnect: () => {
      setBusy(true);
      void tunnel.disconnect().catch(() => {}).finally(() => setBusy(false));
    },
    refresh: async () => {
      const v = await guard(() => servers.refresh());
      if (v) setView(v);
    },
    test: async () => {
      setTesting(true);
      const v = await guard(() => servers.test());
      if (v) setView(v);
      setTesting(false);
    },
    add: async (text) => {
      const v = await guard(() => servers.add(text));
      if (v) setView(v);
      return Boolean(v);
    },
    removeSource: async (id) => {
      const v = await guard(() => servers.removeSource(id));
      if (v) setView(v);
    },
    set: async (s) => {
      const v = await guard(() => servers.set(s));
      if (v) setView(v);
    },
  }), [view, state, history, testing, busy, error]);
  return <Ctx.Provider value={memoizedValue}>{children}</Ctx.Provider>;
}

export function useTunnel(): TunnelContextValue {
  const v = useContext(Ctx);
  if (!v) throw new Error("useTunnel outside TunnelProvider");
  return v;
}
