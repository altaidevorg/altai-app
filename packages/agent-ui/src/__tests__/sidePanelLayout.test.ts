import { describe, expect, it } from "vitest";
import {
  resolveSidePanelChromeLayout,
  SIDE_PANEL_HISTORY_SIDEBAR_MIN_WIDTH,
  SIDE_PANEL_INSPECTOR_SIDEBAR_MIN_WIDTH,
} from "../lib/sidePanelLayout.js";

describe("resolveSidePanelChromeLayout", () => {
  it("never mounts a history rail for sidebar host density", () => {
    const layout = resolveSidePanelChromeLayout({
      variant: "sidebar",
      panelWidth: 1400,
      inspectorOpen: true,
      hasSession: true,
    });
    expect(layout.showHistorySidebar).toBe(false);
    expect(layout.showInspectorSidebar).toBe(true);
    expect(layout.inspectorAvailable).toBe(true);
  });

  it("mounts workspace history rail only at the history breakpoint", () => {
    expect(
      resolveSidePanelChromeLayout({
        variant: "workspace",
        panelWidth: SIDE_PANEL_HISTORY_SIDEBAR_MIN_WIDTH - 1,
        inspectorOpen: false,
        hasSession: true,
      }).showHistorySidebar,
    ).toBe(false);
    expect(
      resolveSidePanelChromeLayout({
        variant: "workspace",
        panelWidth: SIDE_PANEL_HISTORY_SIDEBAR_MIN_WIDTH,
        inspectorOpen: false,
        hasSession: true,
      }).showHistorySidebar,
    ).toBe(true);
  });

  it("docks the inspector only when open, available, and wide enough", () => {
    expect(
      resolveSidePanelChromeLayout({
        variant: "sidebar",
        panelWidth: SIDE_PANEL_INSPECTOR_SIDEBAR_MIN_WIDTH,
        inspectorOpen: true,
        hasSession: false,
      }).showInspectorSidebar,
    ).toBe(false);
    expect(
      resolveSidePanelChromeLayout({
        variant: "sidebar",
        panelWidth: SIDE_PANEL_INSPECTOR_SIDEBAR_MIN_WIDTH,
        inspectorOpen: false,
        hasSession: true,
      }).showInspectorSidebar,
    ).toBe(false);
    expect(
      resolveSidePanelChromeLayout({
        variant: "sidebar",
        panelWidth: SIDE_PANEL_INSPECTOR_SIDEBAR_MIN_WIDTH,
        inspectorOpen: true,
        hasSession: true,
      }).showInspectorSidebar,
    ).toBe(true);
  });
});
