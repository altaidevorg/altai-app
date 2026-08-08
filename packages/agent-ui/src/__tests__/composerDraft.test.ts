import { describe, expect, it } from "vitest";
import {
  appendUniqueByKey,
  applyComposerSlashOutcome,
  basenameForAttach,
  browserFileToAttachment,
  buildComposerCommandSource,
  classifyBrowserFile,
  MAX_PDF_INLINE_BYTES,
  removeAcceptedItems,
  selectionToComposerAttachment,
} from "../lib/composerDraft.js";
import { MAX_TEXT_INLINE } from "../lib/composerAttachments.js";

describe("buildComposerCommandSource", () => {
  it("prefixes first picked command when plain text", () => {
    expect(buildComposerCommandSource("fix bug", ["plan"])).toBe(
      "#plan fix bug",
    );
    expect(buildComposerCommandSource("/help", ["plan"])).toBe("/help");
  });
});

describe("applyComposerSlashOutcome", () => {
  it("handles local outcome", () => {
    expect(
      applyComposerSlashOutcome(
        { kind: "handled", toast: "ok" },
        "x",
      ).abortAsHandled,
    ).toBe(true);
  });

  it("maps send-prompt to marker + prompt", () => {
    const r = applyComposerSlashOutcome(
      { kind: "send-prompt", prompt: "expanded", commandName: "init" },
      "raw",
    );
    expect(r.effectiveText).toBe("expanded");
    expect(r.commandMarker).toBe('<altai-command name="init" />');
  });
});

describe("selection + picks", () => {
  it("builds selection chips", () => {
    const att = selectionToComposerAttachment({
      id: "s1",
      source: "editor",
      text: "const x = 1",
    });
    expect(att.kind).toBe("selection");
    expect(att.source).toBe("editor");
  });

  it("append unique and remove accepted", () => {
    const a = { id: "1" };
    const b = { id: "2" };
    expect(appendUniqueByKey([a], a, (x) => x.id)).toEqual([a]);
    expect(appendUniqueByKey([a], b, (x) => x.id)).toEqual([a, b]);
    expect(removeAcceptedItems([a, b], [a])).toEqual([b]);
  });
});

describe("classifyBrowserFile", () => {
  it("classifies image and rejects oversized pdf/text", () => {
    expect(
      classifyBrowserFile({
        name: "a.png",
        type: "image/png",
        size: 10,
        lastModified: 1,
      }).ok,
    ).toBe(true);
    expect(
      classifyBrowserFile({
        name: "a.pdf",
        type: "application/pdf",
        size: MAX_PDF_INLINE_BYTES + 1,
        lastModified: 1,
      }),
    ).toEqual({ ok: false, reason: "too-large-pdf" });
    expect(
      classifyBrowserFile({
        name: "big.txt",
        type: "text/plain",
        size: MAX_TEXT_INLINE + 1,
        lastModified: 1,
      }),
    ).toEqual({ ok: false, reason: "too-large-text" });
  });

  it("assembles attachment from classification", () => {
    const cls = classifyBrowserFile({
      name: "a.ts",
      type: "text/plain",
      size: 3,
      lastModified: 1,
    });
    expect(cls.ok).toBe(true);
    if (!cls.ok) return;
    const att = browserFileToAttachment(cls, "a.ts", {
      text: "hi\n",
      size: 3,
    });
    expect(att.kind).toBe("text");
    expect(att.text).toBe("hi\n");
  });
});

describe("basenameForAttach", () => {
  it("uses last path segment", () => {
    expect(basenameForAttach("/tmp/proj/a.ts")).toBe("a.ts");
  });
});
