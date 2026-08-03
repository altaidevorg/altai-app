import { WindowControls } from "@/components/WindowControls";
import { IS_MAC } from "@/lib/platform";
import { hasTauriWindowMetadata } from "@/lib/tauriWindow";
import type { SettingsTab } from "@/modules/settings/openSettingsWindow";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useEffect, useState } from "react";
import { normalizeSettingsTab, SettingsContent } from "./SettingsContent";

/**
 * Native settings window for the agent-first ALTAI Studio app. IDE settings
 * deliberately remain an in-IDE tab and do not use this entry point.
 */
function readInitialTab(): SettingsTab {
  if (typeof window === "undefined") return "general";
  const url = new URL(window.location.href);
  return normalizeSettingsTab(url.searchParams.get("tab") ?? undefined, "app");
}

export function SettingsApp() {
  const [active, setActive] = useState<SettingsTab>(readInitialTab);

  useEffect(() => {
    if (!hasTauriWindowMetadata()) return;

    let unlistenPromise: Promise<() => void>;
    try {
      unlistenPromise = getCurrentWebviewWindow().listen<string>(
        "altai:settings-tab",
        (e) => setActive(normalizeSettingsTab(e.payload, "app")),
      );
    } catch (error) {
      console.warn("Could not access the settings window", error);
      return;
    }
    return () => {
      void unlistenPromise.then((un) => un());
    };
  }, []);

  return (
    <div className="flex h-screen flex-col overflow-hidden bg-background text-foreground select-none">
      <header
        data-tauri-drag-region
        className={`flex h-11 shrink-0 items-center border-b border-border/60 bg-card/60 ${
          IS_MAC ? "pr-3 pl-22" : "pr-0 pl-3"
        }`}
      >
        <div className="flex-1" />
        {!IS_MAC && <WindowControls closeOnly />}
      </header>
      <div className="min-h-0 flex-1">
        <SettingsContent
          surface="app"
          active={active}
          onActiveChange={setActive}
        />
      </div>
    </div>
  );
}
