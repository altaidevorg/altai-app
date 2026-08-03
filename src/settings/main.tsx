import "@fontsource/jetbrains-mono/latin-400.css";
import "@fontsource/jetbrains-mono/latin-700.css";
import "@fontsource/jetbrains-mono/cyrillic-400.css";
import "@fontsource/jetbrains-mono/cyrillic-700.css";
import "../styles/globals.css";

import { invoke } from "@tauri-apps/api/core";
import ReactDOM from "react-dom/client";
import { Component, type ErrorInfo, type ReactNode, useEffect } from "react";
import { ThemeProvider } from "@/modules/theme";
import { USE_CUSTOM_WINDOW_CONTROLS } from "@/lib/platform";
import { SettingsApp } from "./SettingsApp";

if (USE_CUSTOM_WINDOW_CONTROLS) {
  document.documentElement.dataset.chrome = "borderless";
}

type SettingsBoundaryState = { error: Error | null };

class SettingsBoundary extends Component<
  { children: ReactNode },
  SettingsBoundaryState
> {
  state: SettingsBoundaryState = { error: null };

  static getDerivedStateFromError(error: Error): SettingsBoundaryState {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("ALTAI settings renderer failed", error, info);
  }

  render() {
    if (!this.state.error) return this.props.children;
    return (
      <main className="flex h-screen w-screen items-center justify-center bg-background p-6 text-foreground">
        <section className="w-full max-w-2xl rounded-xl border border-destructive/40 bg-destructive/[0.06] p-5">
          <h1 className="text-sm font-semibold">Settings could not finish loading</h1>
          <pre className="mt-4 max-h-[50vh] overflow-auto whitespace-pre-wrap rounded-md bg-background/70 p-3 font-mono text-xs leading-relaxed text-destructive">
            {this.state.error.stack || this.state.error.message}
          </pre>
        </section>
      </main>
    );
  }
}

function RendererReady() {
  useEffect(() => {
    document.documentElement.dataset.rendererReady = "true";
    void invoke<boolean>("renderer_ready").catch((error) => {
      console.warn("settings renderer-ready checkpoint failed", error);
    });
  }, []);
  return null;
}

ReactDOM.createRoot(
  document.getElementById("settings-root") as HTMLElement,
).render(
  <SettingsBoundary>
    <ThemeProvider>
      <SettingsApp />
      <RendererReady />
    </ThemeProvider>
  </SettingsBoundary>,
);
