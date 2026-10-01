import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { inTauri } from "./platform";

export interface Available {
  version: string;
  /** Release notes, Markdown as published. */
  notes: string;
  date: string | null;
  /** Too old to keep using: the dialog cannot be put off. */
  required: boolean;
}

export interface UpdateView {
  /** False in a build without the signing key. */
  configured: boolean;
  current: string;
  available: Available | null;
}

const NONE: UpdateView = { configured: false, current: "", available: null };

export const updates = {
  state: () => (inTauri ? invoke<UpdateView>("update_state") : Promise.resolve(NONE)),
  check: () => (inTauri ? invoke<UpdateView>("update_check") : Promise.resolve(NONE)),
  install: () => invoke<void>("update_install"),
  onAvailable: (f: (a: Available) => void): Promise<UnlistenFn> =>
    inTauri ? listen<Available>("update://available", (e) => f(e.payload)) : Promise.resolve(() => {}),
  onProgress: (f: (p: { downloaded: number; total: number | null }) => void): Promise<UnlistenFn> =>
    inTauri ? listen<{ downloaded: number; total: number | null }>("update://progress", (e) => f(e.payload)) : Promise.resolve(() => {}),
};

/** Release notes as list items: Markdown bullets, else non-empty lines. */
export function noteLines(notes: string): string[] {
  return notes
    .split(/\r?\n/)
    .map((l) => l.replace(/^\s*(?:[-*•]|\d+[.)])\s+/, "").replace(/^#+\s*/, "").replace(/\*\*/g, "").trim())
    .filter((l) => l.length > 0)
    .slice(0, 12);
}
