import { useEffect, useState } from "react";
import { support } from "./account";

/** Unread support replies, kept current by the Rust side's watch. */
export function useUnread(): number {
  const [count, setCount] = useState(0);
  useEffect(() => {
    void support.unread().then(setCount, () => {});
    const off = support.onUnread(setCount);
    return () => void off.then((f) => f());
  }, []);
  return count;
}
