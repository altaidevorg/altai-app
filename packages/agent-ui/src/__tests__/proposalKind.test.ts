import { describe, expect, it } from "vitest";
import { proposalKindFromPlanEdit } from "../lib/proposalKind.js";

describe("proposalKindFromPlanEdit", () => {
  it("maps kinds", () => {
    expect(proposalKindFromPlanEdit("create_directory")).toBe("create_directory");
    expect(proposalKindFromPlanEdit("edit", true)).toBe("create_file");
    expect(proposalKindFromPlanEdit("edit")).toBe("edit");
    expect(proposalKindFromPlanEdit("other")).toBe("edit_file");
  });
});
