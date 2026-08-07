/**
 * Pure layout thresholds for the AI side panel chrome.
 * Desktop AiSidePanel and VS Code hosts share these breakpoints so density
 * decisions stay consistent without pulling stores or Resizable into the package.
 */

export type SidePanelVariant = "sidebar" | "workspace";

/** Standalone workspace can mount a persistent history rail at this width. */
export const SIDE_PANEL_HISTORY_SIDEBAR_MIN_WIDTH = 768;

/** Run inspector docks beside the main chat from this width. */
export const SIDE_PANEL_INSPECTOR_SIDEBAR_MIN_WIDTH = 1216;

export type SidePanelChromeLayoutInput = {
  variant: SidePanelVariant;
  panelWidth: number;
  inspectorOpen: boolean;
  hasSession: boolean;
};

export type SidePanelChromeLayout = {
  inspectorAvailable: boolean;
  /**
   * Persistent left history rail (workspace only). IDE/sidebar variants keep
   * history as an overlay destination — never a second left rail.
   */
  showHistorySidebar: boolean;
  /** Docked right inspector column when width + availability allow. */
  showInspectorSidebar: boolean;
};

/**
 * Resolve which side columns the panel shell should mount for a given width.
 */
export function resolveSidePanelChromeLayout(
  input: SidePanelChromeLayoutInput,
): SidePanelChromeLayout {
  const inspectorAvailable = input.hasSession;
  const showHistorySidebar =
    input.variant === "workspace" &&
    input.panelWidth >= SIDE_PANEL_HISTORY_SIDEBAR_MIN_WIDTH;
  const showInspectorSidebar =
    input.panelWidth >= SIDE_PANEL_INSPECTOR_SIDEBAR_MIN_WIDTH &&
    input.inspectorOpen &&
    inspectorAvailable;
  return {
    inspectorAvailable,
    showHistorySidebar,
    showInspectorSidebar,
  };
}
