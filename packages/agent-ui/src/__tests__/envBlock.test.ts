import { describe, expect, it } from "vitest";
import {
  buildEnvBlockFromFacts,
  buildIsolatedWorktreeEnvBlock,
  formatEnvBlock,
  prependEnvBlockToText,
} from "../lib/envBlock.js";

describe("envBlock", () => {
  it("formats and nulls empty", () => {
    expect(formatEnvBlock([])).toBeNull();
    expect(formatEnvBlock(["a: 1"])).toBe("<env>\na: 1\n</env>");
  });

  it("builds from live facts", () => {
    expect(
      buildEnvBlockFromFacts({
        workspaceRoot: "/w",
        cwd: "/w/src",
        activeFile: "a.ts",
        activeTerminalPrivate: true,
      }),
    ).toContain("workspace_root: /w");
  });

  it("isolated worktree + prepend", () => {
    const env = buildIsolatedWorktreeEnvBlock("/tmp/wt", "feat/x");
    expect(env).toContain("isolated-worktree");
    expect(prependEnvBlockToText("hi", env)).toMatch(/<\/env>\n\nhi$/);
    expect(prependEnvBlockToText("hi", null)).toBe("hi");
  });
});
