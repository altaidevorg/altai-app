import { describe, expect, it } from "vitest";
import {
  STATUS_BAR_CHECKPOINTS_TITLE,
  STATUS_BAR_CLOSE_PANEL_LABEL,
} from "../lib/statusBarResidualChrome.js";

describe("statusBarResidualChrome", () => {
  it("exposes residual status-bar titles", () => {
    expect(STATUS_BAR_CLOSE_PANEL_LABEL).toBe("Close AI panel");
    expect(STATUS_BAR_CHECKPOINTS_TITLE).toContain("checkpoints");
  });
});
