import { invoke } from "@tauri-apps/api/core";

export type SettingsTab =
  | "general"
  | "shortcuts"
  | "models"
  | "agents"
  | "skills"
  | "github"
  | "language-servers"
  | "mcp"
  | "hooks"
  | "context"
  | "accessibility"
  | "about";

/**
 * ALTAI Studio (the agent-first app) and ALTAI IDE settings are intentionally
 * different surfaces. This registry remains as a browser/dev fallback.
 */
type OpenImpl = (tab?: SettingsTab) => void;

let openImpl: OpenImpl | null = null;

/**
 * Register the host's settings-opening function. Returns an unregister
 * callback for use as an effect cleanup.
 */
export function registerOpenSettings(impl: OpenImpl): () => void {
  openImpl = impl;
  return () => {
    if (openImpl === impl) openImpl = null;
  };
}

function openNativeSettingsWindow(
  command: "open_settings_window",
  label: "Studio",
  tab?: SettingsTab,
): Promise<void> {
  return invoke<void>(command, { tab: tab ?? null }).catch((error) => {
    if (openImpl) {
      openImpl(tab);
      return;
    }
    if (typeof console !== "undefined") {
      console.warn(`Could not open ALTAI ${label} settings`, error);
    }
    throw error;
  });
}

/** Open (or refocus) the agent-first ALTAI Studio settings window. */
export function openSettingsWindow(tab?: SettingsTab): Promise<void> {
  return openNativeSettingsWindow("open_settings_window", "Studio", tab).catch(
    () => undefined,
  );
}
