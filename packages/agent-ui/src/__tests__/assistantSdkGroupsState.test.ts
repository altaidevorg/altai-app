import { describe, expect, it } from "vitest";
import { buildAssistantSdkGroupsState } from "../lib/assistantSdkGroupsState.js";

describe("buildAssistantSdkGroupsState", () => {
  it("indexes last text and groups parts", () => {
    const state = buildAssistantSdkGroupsState([
      { type: "text", text: "a" },
      { type: "tool-exec", state: "output-available" },
      { type: "text", text: "b" },
    ]);
    expect(state.lastTextPartIdx).toBe(2);
    expect(state.groups.length).toBeGreaterThan(0);
  });
});
