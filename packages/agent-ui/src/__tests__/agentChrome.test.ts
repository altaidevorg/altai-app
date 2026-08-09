import { describe, expect, it } from "vitest";
import {
  applyAgentOverride,
  diffAgentAgainstBase,
  findAgentById,
  newAgentId,
} from "../lib/agentChrome.js";

describe("agentChrome (A6.152)", () => {
  it("generates ids and finds agents", () => {
    expect(newAgentId(() => 1, () => 0.1)).toMatch(/^a-/);
    const agents = [
      { id: "a", name: "A" },
      { id: "b", name: "B" },
    ];
    expect(findAgentById(agents, "b", agents[0]!).name).toBe("B");
    expect(findAgentById(agents, null, agents[0]!).id).toBe("a");
  });

  it("applies and diffs overrides", () => {
    const base = {
      name: "Coder",
      description: "d",
      instructions: "i",
      icon: "coder",
    };
    expect(applyAgentOverride(base, { name: "X" }).name).toBe("X");
    expect(diffAgentAgainstBase(base, { ...base, name: "Y" })).toEqual({
      name: "Y",
    });
  });
});
