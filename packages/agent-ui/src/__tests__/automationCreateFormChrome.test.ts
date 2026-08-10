import { describe, expect, it } from "vitest";
import {
  AUTOMATION_CREATE_SESSION_FALLBACK_TITLE,
  automationCreateStatusText,
  automationCreateSubmitLabel,
} from "../lib/automationCreateFormChrome.js";

describe("automationCreateFormChrome", () => {
  it("builds status and submit labels", () => {
    expect(automationCreateStatusText("c1")).toBe("Schedule is ready");
    expect(automationCreateStatusText(null)).toBe(
      "Select a chat to create one",
    );
    expect(automationCreateSubmitLabel(true)).toBe("Creating…");
    expect(automationCreateSubmitLabel(false)).toBe("Create");
    expect(AUTOMATION_CREATE_SESSION_FALLBACK_TITLE).toBe("New chat");
  });
});
