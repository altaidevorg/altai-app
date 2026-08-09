import { describe, expect, it } from "vitest";
import { joinWorkspaceRelativePath } from "../lib/joinWorkspacePath.js";

describe("joinWorkspaceRelativePath", () => {
  it("joins with trailing slash stripped from root", () => {
    expect(joinWorkspaceRelativePath("/ws/", ".altai/commands/x.md")).toBe(
      "/ws/.altai/commands/x.md",
    );
    expect(joinWorkspaceRelativePath("C:\\ws\\", "src/a.ts")).toBe(
      "C:\\ws/src/a.ts",
    );
  });

  it("keeps root when no trailing separator", () => {
    expect(joinWorkspaceRelativePath("/ws", "a/b")).toBe("/ws/a/b");
  });
});
