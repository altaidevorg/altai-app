import { describe, expect, it } from "vitest";
import {
  advanceCaretAfterDraftChange,
  resolveComposerCaret,
} from "../lib/composerCaretChrome.js";

describe("resolveComposerCaret", () => {
  it("clamps finite selection into the draft", () => {
    expect(resolveComposerCaret(3, 10)).toBe(3);
    expect(resolveComposerCaret(99, 4)).toBe(4);
    expect(resolveComposerCaret(-1, 4)).toBe(4);
  });

  it("falls back to end when selection is missing", () => {
    expect(resolveComposerCaret(undefined, 5)).toBe(5);
    expect(resolveComposerCaret(null, 0)).toBe(0);
  });
});

describe("advanceCaretAfterDraftChange", () => {
  it("jumps to end when typing into an empty draft with stale caret 0", () => {
    expect(advanceCaretAfterDraftChange(0, "/help", 0)).toBe(5);
  });

  it("preserves mid-draft caret when not stuck at zero-growth edge", () => {
    expect(advanceCaretAfterDraftChange(2, "abc", 2)).toBe(2);
  });

  it("resets when draft cleared", () => {
    expect(advanceCaretAfterDraftChange(4, "", 4)).toBe(0);
  });
});
