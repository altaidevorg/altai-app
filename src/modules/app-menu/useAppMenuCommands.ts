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
    const unlisten = getCurrentWebviewWindow().listen<AppMenuCommand>(
      "altai:menu-command",
      (event) => latest.current(event.payload),
    );
    return () => {
      void unlisten.then((dispose) => dispose());
    };
  }, []);
}
