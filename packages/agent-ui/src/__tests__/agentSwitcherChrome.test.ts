import { describe, expect, it } from "vitest";
import {
  partitionAgentsForSwitcher,
  resolveSwitcherActiveAgent,
} from "../lib/agentSwitcherChrome.js";

describe("partitionAgentsForSwitcher", () => {
  it("splits built-in / ml / custom", () => {
    const rows = [
      { id: "coder", builtIn: true },
      { id: "isan-ml", builtIn: true },
      { id: "mine", builtIn: false },
    ];
    const parts = partitionAgentsForSwitcher(
      rows,
      (id) => id === "isan-ml",
    );
    expect(parts.builtIn.map((a) => a.id)).toEqual(["coder"]);
    expect(parts.mlAgents.map((a) => a.id)).toEqual(["isan-ml"]);
    expect(parts.custom.map((a) => a.id)).toEqual(["mine"]);
  });
});

describe("resolveSwitcherActiveAgent", () => {
  it("prefers full list active id", () => {
    const all = [
      { id: "a" },
      { id: "b" },
    ];
    const enabled = [{ id: "b" }];
    expect(resolveSwitcherActiveAgent(all, enabled, "a")?.id).toBe("a");
    expect(resolveSwitcherActiveAgent(all, enabled, "missing")?.id).toBe("b");
  });
});
