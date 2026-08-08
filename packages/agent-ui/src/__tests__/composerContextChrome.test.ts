import { describe, expect, it } from "vitest";
import {
  addContextItem,
  basenamePath,
  clipContextText,
  composeRunPrompt,
  countLines,
  formatTextContextBlocks,
  listOpenableContextItems,
  removeContextItem,
  resolveContextOpenUri,
  toContextChips,
  toRunAttachments,
  type ComposerContextItem,
} from "../lib/composerContextChrome.js";

const sampleFile: ComposerContextItem = {
  id: "f1",
  kind: "file",
  uri: "file:///tmp/proj/a.ts",
  name: "a.ts",
  path: "/tmp/proj/a.ts",
};

const sampleSelection: ComposerContextItem = {
  id: "s1",
  kind: "selection",
  path: "/tmp/proj/a.ts",
  text: "const x = 1;\nconst y = 2;",
  lines: 2,
};

describe("composerContext helpers", () => {
  it("dedupes by key and removes by id", () => {
    const withOne = addContextItem([], sampleFile);
    const withDup = addContextItem(withOne, {
      ...sampleFile,
      id: "f2",
    });
    expect(withDup).toHaveLength(1);
    expect(removeContextItem(withDup, "f1")).toHaveLength(0);
  });

  it("separates file attachments from text blocks", () => {
    const items = [sampleFile, sampleSelection];
    expect(toRunAttachments(items)).toEqual([
      { uri: "file:///tmp/proj/a.ts", name: "a.ts" },
    ]);
    const blocks = formatTextContextBlocks(items);
    expect(blocks).toContain("```context selection");
    expect(blocks).toContain("const x = 1");
    expect(blocks).not.toContain("file:///tmp/proj/a.ts");
  });

  it("composes prompt with context prefix", () => {
    const result = composeRunPrompt("fix this", [sampleSelection]);
    expect(result.prompt.startsWith("```context selection")).toBe(true);
    expect(result.prompt.endsWith("fix this")).toBe(true);
    expect(result.attachments).toEqual([]);
  });

  it("maps chips and utilities", () => {
    expect(countLines("a\nb\n")).toBe(2);
    expect(basenamePath("/a/b/c.ts")).toBe("c.ts");
    expect(clipContextText("hi").truncated).toBe(false);
    expect(toContextChips([sampleFile, sampleSelection])).toEqual([
      { kind: "file", name: "a.ts", lines: 0 },
      { kind: "selection", source: "editor", lines: 2 },
    ]);
  });

  it("resolves openable context URIs", () => {
    expect(resolveContextOpenUri(sampleFile)).toBe("file:///tmp/proj/a.ts");
    expect(resolveContextOpenUri(sampleSelection)).toBeNull();
    expect(
      resolveContextOpenUri({
        ...sampleSelection,
        uri: "file:///tmp/proj/a.ts",
      }),
    ).toBe("file:///tmp/proj/a.ts");
    expect(
      listOpenableContextItems([
        sampleFile,
        sampleSelection,
        {
          id: "d1",
          kind: "diff",
          name: "diff",
          text: "x",
          lines: 1,
        },
      ]),
    ).toEqual([{ id: "f1", label: "a.ts", uri: "file:///tmp/proj/a.ts" }]);
  });
});
