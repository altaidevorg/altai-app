import { describe, expect, it } from "vitest";
import {
  DEFAULT_COMPOSER_AGENT_ID,
  applyAgentPromptPrefix,
  canMountAgentPicker,
  resolveComposerAgent,
} from "../lib/agentPickerChrome.js";

describe("agentPickerChrome", () => {
  it("resolves default when unknown", () => {
    expect(resolveComposerAgent("nope").id).toBe(DEFAULT_COMPOSER_AGENT_ID);
    expect(resolveComposerAgent(null).id).toBe(DEFAULT_COMPOSER_AGENT_ID);
  });

  it("resolves known agents", () => {
    expect(resolveComposerAgent("builtin:reviewer").name).toBe("Code Reviewer");
  });

  it("gates the picker on prefs", () => {
    expect(canMountAgentPicker({ agentPickerEnabled: true })).toBe(true);
    expect(canMountAgentPicker({ agentPickerEnabled: false })).toBe(false);
  });

  it("prefixes the prompt once", () => {
    const agent = resolveComposerAgent("builtin:architect");
    const once = applyAgentPromptPrefix("fix the tests", agent);
    expect(once).toContain("[ALTAI agent: Architect]");
    expect(once).toContain("fix the tests");
    const twice = applyAgentPromptPrefix(once, agent);
    expect(twice).toBe(once);
  });
});
