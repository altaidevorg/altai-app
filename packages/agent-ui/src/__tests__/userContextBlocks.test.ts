import { describe, expect, it } from "vitest";
import { stripUserContextBlocks } from "../lib/userContextBlocks.js";

describe("stripUserContextBlocks", () => {
  it("returns plain text unchanged", () => {
    expect(stripUserContextBlocks("hello world")).toEqual({
      text: "hello world",
      chips: [],
    });
  });

  it("extracts file, selection, and snippet chips", () => {
    const raw = [
      `<file name="src/a.ts">\nconst x = 1;\n</file>`,
      `<selection source="editor">\nline one\nline two\n</selection>`,
      `<snippet name="fix">\nbody\n</snippet>`,
      "please review",
    ].join("\n");
    const result = stripUserContextBlocks(raw);
    expect(result.text).toBe("please review");
    // Chip order follows strip replace-pass order (selection → file → …),
    // not document order — matches Desktop AiChat historical behaviour.
    expect(result.chips).toEqual([
      { kind: "selection", source: "editor", lines: 2 },
      { kind: "file", name: "src/a.ts", lines: 1 },
      { kind: "snippet", name: "fix" },
    ]);
  });

  it("supports terminal, git-diff, and folder markers", () => {
    const result = stripUserContextBlocks(
      [
        `<terminal-context name="zsh">\na\nb\n</terminal-context>`,
        `<git-diff>\n+x\n</git-diff>`,
        `<folder name="src">\nfile\n</folder>`,
      ].join(""),
    );
    expect(result.text).toBe("");
    expect(result.chips).toEqual([
      { kind: "terminal", name: "zsh", lines: 2 },
      { kind: "diff", name: "Working tree diff", lines: 1 },
      { kind: "folder", name: "src", lines: 1 },
    ]);
  });
});
