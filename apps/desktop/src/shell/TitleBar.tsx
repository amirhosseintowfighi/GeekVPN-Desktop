import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useState } from "react";
import { Icon, type IconName } from "../design-system/Icon";
import { inTauri, type DesktopOs } from "../lib/platform";

function CaptionButton({ icon, label, onClick, pressed }: { icon: IconName; label: string; onClick: () => void; pressed?: boolean }) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      aria-pressed={pressed}
      onClick={onClick}
      className={`flex h-[34px] w-11 items-center justify-center rounded-lg text-white transition-colors hover:bg-white/15 ${
        pressed ? "opacity-100" : "opacity-85"
      } ${icon === "x" ? "hover:bg-[#D93F48]" : ""}`}
    >
      <Icon name={icon} size={16} color="#FFFFFF" />
    </button>
  );
}

/**
 * The frameless window's title bar. Windows and Linux get the design's own
 * caption buttons, mirrored to the left as RTL Windows does; macOS keeps its
 * native traffic lights (tauri.macos.conf.json) and only the pin button is ours.
 */
export function TitleBar({ os }: { os: DesktopOs }) {
  const [maximized, setMaximized] = useState(false);
  const [pinned, setPinned] = useState(false);

  useEffect(() => {
    if (!inTauri) return;
    const win = getCurrentWindow();
    let alive = true;
    const sync = () => win.isMaximized().then((m) => alive && setMaximized(m));
    void sync();
    const off = win.onResized(() => void sync());
    return () => {
      alive = false;
      void off.then((f) => f());
    };
  }, []);

  const win = inTauri ? getCurrentWindow() : null;
  const togglePin = () => {
    const next = !pinned;
    setPinned(next);
    void win?.setAlwaysOnTop(next);
  };

  return (
    <div data-tauri-drag-region className="absolute inset-x-0 top-0 z-20 flex h-[38px] items-center ps-4 pe-1.5">
      <span data-tauri-drag-region className="h-full flex-1" />
      <div dir="ltr" className={`flex ${os === "macos" ? "absolute left-[84px] top-0.5" : ""}`}>
        <CaptionButton icon="pin" label={pinned ? "برداشتن سنجاق" : "سنجاق (همیشه رو)"} pressed={pinned} onClick={togglePin} />
        {os !== "macos" && (
          <>
            <CaptionButton icon="min" label="کوچک‌سازی" onClick={() => void win?.minimize()} />
            <CaptionButton
              icon={maximized ? "restore" : "max"}
              label={maximized ? "بازگرداندن" : "بزرگ‌سازی"}
              onClick={() => void win?.toggleMaximize()}
            />
            <CaptionButton icon="x" label="بستن" onClick={() => void win?.close()} />
          </>
        )}
      </div>
    </div>
  );
}
