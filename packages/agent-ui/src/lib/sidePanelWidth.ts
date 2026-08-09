/**
 * Pure bounds helpers for resizable history/inspector side panels (A6.133).
 * Hosts supply storage I/O; this package never imports vscode or Tauri.
 */

export const HISTORY_PANEL_MIN_WIDTH = 176;
export const HISTORY_PANEL_MAX_WIDTH = 360;
export const INSPECTOR_PANEL_MIN_WIDTH = 240;
export const INSPECTOR_PANEL_MAX_WIDTH = 480;

export const HISTORY_PANEL_WIDTH_KEY = "altai.ai.historyPanel.width";
export const INSPECTOR_PANEL_WIDTH_KEY = "altai.ai.inspectorPanel.width";

export function clampPanelWidth(
  width: number,
  min: number,
  max: number,
): number {
  if (!Number.isFinite(width)) {
    return min;
  }
  return Math.min(max, Math.max(min, width));
}

/**
 * Parse a stored panel width string (e.g. localStorage) with min/max bounds.
 */
export function parsePanelWidth(
  raw: string | null | undefined,
  fallback: number,
  min: number,
  max: number,
): number {
  const parsed = Number.parseInt(raw ?? "", 10);
  if (!Number.isFinite(parsed)) {
    return clampPanelWidth(fallback, min, max);
  }
  return clampPanelWidth(parsed, min, max);
}

export function serializePanelWidth(
  width: number,
  min: number,
  max: number,
): string | null {
  if (width <= 0 || !Number.isFinite(width)) {
    return null;
  }
  return String(Math.round(clampPanelWidth(width, min, max)));
}
