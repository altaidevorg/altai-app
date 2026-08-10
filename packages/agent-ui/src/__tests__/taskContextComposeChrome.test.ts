import { describe, expect, it } from "vitest";
import {
  composePromptWithSelectedContext,
  wrapContextFileBlock,
  wrapTerminalContextBlock,
  wrapWorkingTreeDiffBlock,
} from "../lib/taskContextComposeChrome.js";

describe("wrapContextFileBlock", () => {
  it("caps content", () => {
    expect(wrapContextFileBlock("/a.ts", "abcdef", 3)).toBe(
      `<context-file path="/a.ts">\nabc\n</context-file>`,
    );
  });
});

describe("wrapTerminalContextBlock", () => {
  it("returns null when blank", () => {
    expect(wrapTerminalContextBlock("  ")).toBeNull();
  });
  it("wraps trimmed output", () => {
    expect(wrapTerminalContextBlock(" hi ")).toBe(
      "<terminal-context>\nhi\n</terminal-context>",
    );
  });
});

describe("wrapWorkingTreeDiffBlock", () => {
  it("adds truncated attr", () => {
    expect(wrapWorkingTreeDiffBlock("diff", true)).toBe(
      `<working-tree-diff truncated="true">\ndiff\n</working-tree-diff>`,
    );
  });
});

describe("composePromptWithSelectedContext", () => {
  it("keeps raw prompt when no blocks", () => {
    expect(composePromptWithSelectedContext("  hi  ", [])).toBe("  hi  ");
  });
  it("joins blocks", () => {
    expect(
      composePromptWithSelectedContext("go", ["<a/>", "<b/>"]),
    ).toBe("go\n\n<selected-context>\n<a/>\n\n<b/>\n</selected-context>");
  });
});
