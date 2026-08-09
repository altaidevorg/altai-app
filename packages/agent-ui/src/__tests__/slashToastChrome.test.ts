import { describe, expect, it } from "vitest";
import {
  agentSettingsToast,
  switchedAgentToast,
} from "../lib/slashToastChrome.js";

describe("slash toast chrome", () => {
  it("formats agent switch and settings toasts", () => {
    expect(switchedAgentToast("Coder")).toBe("Switched to Coder");
    expect(agentSettingsToast(true)).toMatch(/not found/);
    expect(agentSettingsToast(false)).toBe("Opened agent settings");
  });
});
