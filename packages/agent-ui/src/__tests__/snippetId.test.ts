import { describe, expect, it } from "vitest";
import { newSnippetId } from "../lib/snippetId.js";

describe("newSnippetId", () => {
  it("formats with injectables", () => {
    expect(newSnippetId(() => 1, () => 0.25)).toMatch(/^sn-/);
  });
});
