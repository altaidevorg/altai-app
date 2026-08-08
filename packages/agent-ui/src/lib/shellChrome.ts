/**
 * Pure helpers for side-chat shell chrome density (A6.74).
 * Hosts own layout; no vscode/Tauri imports.
 */

import type { AltaiSurfaceId } from "./surfaceTabsChrome.js";

export type ShellSurface = AltaiSurfaceId;

/** Desktop sidebar has no Chat | Operations | Settings text tabs. */
export function shouldShowSurfaceTextTabs(): boolean {
  return false;
}

/** Settings gear is pressed when the Settings surface is open. */
export function settingsGearPressed(surface: ShellSurface): boolean {
  return surface === "settings";
}

/**
 * Toggle Settings: open when closed, return to chat when already open
 * (Desktop workspace gear opens settings; activity-bar density toggles).
 */
export function nextSurfaceAfterSettingsToggle(
  surface: ShellSurface,
): ShellSurface {
  return surface === "settings" ? "chat" : "settings";
}

/** Row label for the compact host status chip (empty → omit chip). */
export function compactHostStatusLabel(
  status: string,
  message?: string,
): string | null {
  if (status === "ready") {
    return null;
  }
  if (status === "starting" || status === "connecting") {
    return "Starting…";
  }
  if (status === "error" || status === "crashed") {
    return message?.trim() ? "Host error" : "Error";
  }
  if (status === "stopped" || status === "idle") {
    return "Host offline";
  }
  return status;
}
