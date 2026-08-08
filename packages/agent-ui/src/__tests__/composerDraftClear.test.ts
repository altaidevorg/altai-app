import { describe, expect, it } from "vitest";
import { clearComposerDraftAfterAccept } from "../lib/composerDraftClear.js";
import type { ComposerFileAttachment } from "../lib/composerAttachments.js";
import type { ComposerSnippet } from "../lib/composerSnippets.js";

const file = (id: string): ComposerFileAttachment => ({
  id,
  name: `${id}.txt`,
  kind: "text",
  mediaType: "text/plain",
  text: "x",
  size: 1,
});

const snip = (id: string): ComposerSnippet => ({
  id,
  handle: id,
  name: id,
  description: "",
  content: "body",
});

describe("clearComposerDraftAfterAccept", () => {
  it("clears the full draft when value revision is unchanged", () => {
    const a = file("a");
    const accepted = {
      valueRevision: 1,
      value: "hello",
      files: [a],
      snippets: [snip("s1")],
      commands: [{ name: "fix" }],
    };
    const cleared = clearComposerDraftAfterAccept(accepted, accepted);
    expect(cleared).toEqual({
      value: "",
      files: [],
      snippets: [],
      commands: [],
    });
  });

  it("keeps residual typed text when the field changed in flight", () => {
    const a = file("a");
    const b = file("b");
    const s1 = snip("s1");
    const s2 = snip("s2");
    const cmdFix = { name: "fix" };
    const cmdHelp = { name: "help" };
    const accepted = {
      valueRevision: 1,
      value: "hello",
      files: [a],
      snippets: [s1],
      commands: [cmdFix],
    };
    const current = {
      valueRevision: 2,
      value: "hello extra",
      files: [a, b],
      snippets: [s1, s2],
      commands: [cmdFix, cmdHelp],
    };
    const cleared = clearComposerDraftAfterAccept(current, accepted);
    expect(cleared.value).toBe("extra");
    expect(cleared.files).toEqual([b]);
    expect(cleared.snippets).toEqual([s2]);
    expect(cleared.commands).toEqual([cmdHelp]);
  });

  it("keeps entire current value when it diverged (not a prefix)", () => {
    const accepted = {
      valueRevision: 1,
      value: "old",
      files: [],
      snippets: [],
      commands: [],
    };
    const current = {
      valueRevision: 2,
      value: "rewritten",
      files: [],
      snippets: [],
      commands: [],
    };
    expect(clearComposerDraftAfterAccept(current, accepted).value).toBe(
      "rewritten",
    );
  });
});
