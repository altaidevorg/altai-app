import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useEffect, useRef } from "react";

export type AppMenuCommand = {
  id: string;
  path?: string;
};

/**
 * Connects the native desktop menu to the currently focused ALTAI window.
 * The ref keeps the Tauri listener stable while always invoking the latest
 * React callback.
 */
export function useAppMenuCommands(
  handler: (command: AppMenuCommand) => void,
): void {
  const latest = useRef(handler);
  latest.current = handler;

  useEffect(() => {
    // `pnpm dev` can render the welcome screen in a regular browser, where
    // Tauri's window metadata does not exist. Native menus are desktop-only,
    // so there is nothing to subscribe to in that environment.
    if (!isTauri()) return;

    let disposed = false;
    let unlisten: (() => void) | null = null;
    const subscription = getCurrentWebviewWindow()
      .listen<AppMenuCommand>("altai:menu-command", (event) => {
        latest.current(event.payload);
      })
      .then((dispose) => {
        if (disposed) dispose();
        else unlisten = dispose;
      })
      .catch((error) => {
        console.warn("Could not subscribe to native app menu commands", error);
      });

    return () => {
      disposed = true;
      unlisten?.();
      void subscription;
    };
  }, []);
}
