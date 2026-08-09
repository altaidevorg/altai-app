import { describe, expect, it } from "vitest";
import { applySessionWorkspaceTarget } from "../lib/sessionWorkspaceTarget.js";

describe("applySessionWorkspaceTarget", () => {
  it("patches matching row", () => {
    const out = applySessionWorkspaceTarget(
      [{ id: "a", updatedAt: 1, workspacePath: null }],
      "a",
      { path: "/w", kind: "local" },
      2,
    );
    expect(out[0]).toMatchObject({
      workspacePath: "/w",
      workspaceKind: "local",
      updatedAt: 2,
    });
  });
});
