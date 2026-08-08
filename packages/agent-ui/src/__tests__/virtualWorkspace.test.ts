import { describe, expect, it } from "vitest";
import { isVirtualOnlyWorkspace } from "../lib/virtualWorkspace.js";

describe("isVirtualOnlyWorkspace", () => {
  it("false for empty or file roots", () => {
    expect(isVirtualOnlyWorkspace([])).toBe(false);
    expect(isVirtualOnlyWorkspace([{ scheme: "file", fsPath: "/a" }])).toBe(
      false,
    );
  });
  it("true when all folders are virtual-only", () => {
    expect(
      isVirtualOnlyWorkspace([{ scheme: "vscode-vfs", fsPath: "" }]),
    ).toBe(true);
    expect(isVirtualOnlyWorkspace([{ scheme: "untitled" }])).toBe(true);
  });
});
