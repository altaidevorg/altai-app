import { describe, expect, it } from "vitest";
import {
  COMPOSER_DRAFT_DEBOUNCE_MS,
  shouldPersistComposerDraftImmediately,
} from "../lib/composerDraftPersist.js";

describe("composerDraftPersist", () => {
  it("exports debounce constant", () => {
    expect(COMPOSER_DRAFT_DEBOUNCE_MS).toBe(200);
  });
  it("flushes empty drafts immediately", () => {
    expect(shouldPersistComposerDraftImmediately("")).toBe(true);
    expect(shouldPersistComposerDraftImmediately("hi")).toBe(false);
  });
});
