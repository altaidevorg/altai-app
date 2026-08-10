import { describe, expect, it } from "vitest";
import {
  MODEL_SETTINGS_LABEL,
  PLAN_RESTORE_FALLBACK_ERROR,
  STOP_GENERATING_TITLE,
} from "../lib/miscAiChrome.js";

describe("miscAiChrome", () => {
  it("exposes misc chrome labels", () => {
    expect(MODEL_SETTINGS_LABEL).toBe("Model settings");
    expect(STOP_GENERATING_TITLE).toBe("Stop generating");
    expect(PLAN_RESTORE_FALLBACK_ERROR).toContain("restore");
  });
});
