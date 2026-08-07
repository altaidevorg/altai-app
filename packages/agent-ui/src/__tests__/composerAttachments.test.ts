import { describe, expect, it } from "vitest";
import {
  boundContextText,
  buildTextContextAttachment,
  estimateComposerContextTokens,
  hasComposerDraft,
  hasNativeBinaryAttachment,
  upsertComposerAttachment,
  type ComposerFileAttachment,
} from "../lib/composerAttachments.js";

describe("boundContextText", () => {
  it("truncates with marker past the limit", () => {
    const raw = "x".repeat(100);
    const bounded = boundContextText(raw, 40);
    expect(bounded.length).toBeLessThanOrEqual(40 + 20);
    expect(bounded.endsWith("…[truncated]")).toBe(true);
  });
});

describe("buildTextContextAttachment", () => {
  it("returns null for empty text", () => {
    expect(
      buildTextContextAttachment({
        kind: "terminal",
        name: "t",
        text: "  \n",
      }),
    ).toBeNull();
  });

  it("builds a plain-text attachment", () => {
    const att = buildTextContextAttachment({
      kind: "diff",
      name: "wt",
      text: "  +a\n  ",
    });
    expect(att?.kind).toBe("diff");
    expect(att?.text).toBe("+a");
    expect(att?.id).toBe("context-diff-wt");
  });
});

describe("upsertComposerAttachment", () => {
  it("replaces by id", () => {
    const a: ComposerFileAttachment = {
      id: "1",
      name: "a",
      kind: "text",
      mediaType: "text/plain",
      text: "old",
      size: 3,
    };
    const b = { ...a, text: "new", size: 3 };
    expect(upsertComposerAttachment([a], b)).toEqual([b]);
  });
});

describe("draft / attachment flags", () => {
  it("detects native binaries and drafts", () => {
    expect(
      hasNativeBinaryAttachment([{ kind: "image" }, { kind: "text" }]),
    ).toBe(true);
    expect(hasComposerDraft({ value: "  ", files: [] })).toBe(false);
    expect(hasComposerDraft({ value: "hi", files: [] })).toBe(true);
    expect(estimateComposerContextTokens({ files: [], snippets: [] })).toBe(0);
    expect(
      estimateComposerContextTokens({
        files: [{ kind: "text", text: "abcd" }],
        snippets: [{ content: "efgh" }],
      }),
    ).toBe(2);
  });
});
