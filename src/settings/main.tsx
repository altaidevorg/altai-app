import "@fontsource/jetbrains-mono/latin-400.css";
import "@fontsource/jetbrains-mono/latin-700.css";
import "@fontsource/jetbrains-mono/cyrillic-400.css";
import "@fontsource/jetbrains-mono/cyrillic-700.css";
import "../styles/globals.css";

import ReactDOM from "react-dom/client";
import { ThemeProvider } from "@/modules/theme";
import { IS_WINDOWS, USE_CUSTOM_WINDOW_CONTROLS } from "@/lib/platform";
import { SettingsApp } from "./SettingsApp";

if (USE_CUSTOM_WINDOW_CONTROLS) {
  document.documentElement.dataset.chrome = IS_WINDOWS
    ? "windows-frameless"
    : "borderless";
}

ReactDOM.createRoot(
  document.getElementById("settings-root") as HTMLElement,
).render(
  <ThemeProvider>
    <SettingsApp />
  </ThemeProvider>,
);
