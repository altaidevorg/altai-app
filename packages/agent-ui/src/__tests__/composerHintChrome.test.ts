import { describe, expect, it } from "vitest";
import {
  formatComposerHintLine,
  listComposerAffordances,
} from "../lib/composerHintChrome.js";

describe("composerHintChrome", () => {
  it("lists slash, snippet, and file affordances", () => {
    expect(listComposerAffordances().map((h) => h.glyph)).toEqual([
      "/",
      "#",
      "@",
    ]);
  });

  it("formats the join line", () => {
    expect(formatComposerHintLine()).toMatch(/\/ commands/);
    expect(formatComposerHintLine()).toMatch(/# snippets/);
    expect(formatComposerHintLine()).toMatch(/@ files/);
  });
});
