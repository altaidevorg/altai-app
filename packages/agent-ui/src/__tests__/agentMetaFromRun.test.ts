import { describe, expect, it } from "vitest";
import { projectAgentMetaFromRun } from "../lib/agentMetaFromRun.js";

describe("projectAgentMetaFromRun", () => {
  it("returns null when no run", () => {
    expect(projectAgentMetaFromRun(undefined, () => null)).toBeNull();
  });

  it("maps failed completed run to error", () => {
    const m = projectAgentMetaFromRun(
      {
        completed: true,
        status: "streaming",
        step: "x",
        outcome: { kind: "failed" },
        tokens: { input: 1, output: 2, cached: 0 },
        subagents: [],
      },
      () => "boom",
    );
    expect(m).toMatchObject({
      status: "error",
      step: null,
      error: "boom",
      tokens: { inputTokens: 1, outputTokens: 2, cachedInputTokens: 0 },
    });
  });

  it("keeps live status while running", () => {
    const m = projectAgentMetaFromRun(
      {
        completed: false,
        status: "tool",
        step: "read",
        tokens: { input: 0, output: 0, cached: 0 },
        subagents: ["a"],
      },
      () => null,
    );
    expect(m).toMatchObject({ status: "tool", step: "read", activeSubagents: ["a"] });
  });
});
