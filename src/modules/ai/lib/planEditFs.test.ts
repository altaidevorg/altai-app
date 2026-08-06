import { describe, expect, it, vi } from "vitest";
import {
  applyPlanEditMutation,
  proposalKindFromPlanEdit,
  restorePlanEditMutation,
  type PlanEditFs,
} from "./planEditFs";

function mockFs(): PlanEditFs & {
  writeFile: ReturnType<typeof vi.fn>;
  createDir: ReturnType<typeof vi.fn>;
  delete: ReturnType<typeof vi.fn>;
} {
  return {
    writeFile: vi.fn(async () => undefined),
    createDir: vi.fn(async () => undefined),
    delete: vi.fn(async () => undefined),
  };
}

describe("planEditFs", () => {
  it("maps kinds", () => {
    expect(proposalKindFromPlanEdit("create_directory")).toBe("create_directory");
    expect(proposalKindFromPlanEdit("edit", true)).toBe("create_file");
    expect(proposalKindFromPlanEdit("edit")).toBe("edit");
  });

  it("applies file and directory mutations", async () => {
    const fs = mockFs();
    await applyPlanEditMutation(fs, {
      kind: "edit_file",
      path: "a.ts",
      proposedContent: "x",
    });
    expect(fs.writeFile).toHaveBeenCalledWith("a.ts", "x", {
      source: "ai-plan-review",
    });
    await applyPlanEditMutation(fs, {
      kind: "create_directory",
      path: "src/new",
      proposedContent: "",
    });
    expect(fs.createDir).toHaveBeenCalledWith("src/new", {
      source: "ai-plan-review",
    });
  });

  it("restores new files by delete and edits by rewrite", async () => {
    const fs = mockFs();
    await restorePlanEditMutation(fs, {
      kind: "edit_file",
      path: "n.ts",
      proposedContent: "new",
      originalContent: "",
      isNewFile: true,
    });
    expect(fs.delete).toHaveBeenCalled();
    await restorePlanEditMutation(fs, {
      kind: "edit_file",
      path: "e.ts",
      proposedContent: "new",
      originalContent: "old",
      isNewFile: false,
    });
    expect(fs.writeFile).toHaveBeenCalledWith("e.ts", "old", {
      source: "ai-plan-restore",
    });
  });
});
