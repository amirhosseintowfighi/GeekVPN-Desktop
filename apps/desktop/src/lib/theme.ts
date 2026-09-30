export type ThemeChoice = "light" | "dark" | "system";

const KEY = "geek.theme";

/**
 * The saved choice. Browser storage is only a convenience here: if it is
 * blocked or empty the app simply follows the system, which is the default.
 */
export function loadTheme(): ThemeChoice {
  try {
    const v = localStorage.getItem(KEY);
    return v === "light" || v === "dark" ? v : "system";
  } catch {
    return "system";
  }
}

/** "system" removes the attribute so tokens.css's media query decides. */
export function applyTheme(choice: ThemeChoice, root: HTMLElement = document.documentElement): void {
  if (choice === "system") delete root.dataset.theme;
  else root.dataset.theme = choice;
  try {
    if (choice === "system") localStorage.removeItem(KEY);
    else localStorage.setItem(KEY, choice);
  } catch {
    // Nothing to do: the attribute above already applied it for this run.
  }
}
