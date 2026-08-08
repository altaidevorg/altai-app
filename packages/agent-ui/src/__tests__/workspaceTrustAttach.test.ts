import { describe, expect, it } from "vitest";
import {
  includeUriInWorkspaceProblemsAttach,
  isWorkspaceNotTrustedError,
} from "../lib/workspaceTrustAttach.js";

describe("workspaceTrustAttach", () => {
  it("detects trust errors", () => {
    expect(isWorkspaceNotTrustedError(new Error("workspace_not_trusted"))).toBe(
      true,
    );
    expect(isWorkspaceNotTrustedError("ok")).toBe(false);
  });
  it("filters multi-root problems attach", () => {
    expect(
      includeUriInWorkspaceProblemsAttach({
        uriFolderUri: "a",
        preferredFolderUri: "b",
      }),
    ).toBe(false);
    expect(
      includeUriInWorkspaceProblemsAttach({
        uriFolderUri: "a",
        preferredFolderUri: "a",
      }),
    ).toBe(true);
  });
});
