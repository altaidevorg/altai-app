import { describe, expect, it } from "vitest";
import {
  INSPECTOR_ACTIVITY_TITLE,
  INSPECTOR_CHANGES_EMPTY,
  SIDE_PANEL_CLOSE_LABEL,
  SIDE_PANEL_SETTINGS_LABEL,
} from "../lib/sidePanelChrome.js";

describe("sidePanelChrome", () => {
  it("exposes shell and inspector labels", () => {
    expect(SIDE_PANEL_CLOSE_LABEL).toBe("Close panel");
    expect(SIDE_PANEL_SETTINGS_LABEL).toContain("ALTAI");
    expect(INSPECTOR_ACTIVITY_TITLE).toBe("Activity");
    expect(INSPECTOR_CHANGES_EMPTY).toContain("No changes");
  });
});
