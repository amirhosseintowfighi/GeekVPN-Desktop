import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { polyfillCountryFlagEmojis } from "country-flag-emoji-polyfill";
import flagFont from "country-flag-emoji-polyfill/dist/TwemojiCountryFlags.woff2?url";
import "./styles/index.css";

// Windows has no flag emoji in its own font: its servers' 🇩🇪 would show as
// "DE". Only flags, from the font bundled with the app, nothing from a CDN.
polyfillCountryFlagEmojis("Twemoji Country Flags", flagFont);

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
