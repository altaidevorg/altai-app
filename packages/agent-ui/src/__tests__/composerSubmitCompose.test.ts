import { describe, expect, it } from "vitest";
import {
  composeComposerSubmitText,
  extractComposerMultimodalParts,
  formatComposerFileBlocks,
  mergeSnippetBlocks,
} from "../lib/composerSubmitCompose.js";

const SNIP = {
  id: "1",
  handle: "pr",
  name: "PR",
  description: "",
  content: "review me",
};

describe("formatComposerFileBlocks", () => {
  it("emits ordered XML markers", () => {
    const blocks = formatComposerFileBlocks([
      {
        kind: "terminal",
        name: "sh",
        mediaType: "text/plain",
        text: "ok",
      },
      {
        kind: "text",
        name: "a.ts",
        mediaType: "text/plain",
        text: "x",
      },
    ]);
    expect(blocks[0]).toContain("<terminal-context");
    expect(blocks[1]).toContain('<file name="a.ts"');
  });
});

describe("mergeSnippetBlocks", () => {
  it("dedupes by handle preferring picks", () => {
    const merged = mergeSnippetBlocks({
      picked: [SNIP],
      tokenBlocks: ['<snippet name="pr">\ntoken\n</snippet>'],
    });
    expect(merged).toHaveLength(1);
    expect(merged[0]).toContain("review me");
  });
});

describe("composeComposerSubmitText", () => {
  it("joins marker, snippets, files, body", () => {
    const text = composeComposerSubmitText({
      commandMarker: '<altai-command name="init" />',
      effectiveText: "hello #pr",
      catalog: [SNIP],
      files: [
        {
          kind: "diff",
          name: "wt",
          mediaType: "text/plain",
          text: "+line",
        },
      ],
    });
    expect(text).toContain('<altai-command name="init" />');
    expect(text).toContain('<snippet name="pr">');
    expect(text).toContain("<git-diff");
    expect(text).toContain("hello");
    expect(text).not.toContain("#pr");
  });
});

describe("extractComposerMultimodalParts", () => {
  it("pulls images and pdf data URLs", () => {
    const parts = extractComposerMultimodalParts([
      {
        kind: "image",
        url: "data:image/png;base64,abc",
        mediaType: "image/png",
        name: "i.png",
      },
      {
        kind: "pdf",
        url: "data:application/pdf;base64,xyz",
        mediaType: "application/pdf",
        name: "d.pdf",
      },
    ]);
    expect(parts.imageUrls).toEqual(["data:image/png;base64,abc"]);
    expect(parts.documents[0]?.data).toBe("xyz");
  });
});
