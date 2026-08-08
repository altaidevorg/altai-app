import { describe, expect, it } from "vitest";
import { useComposerSuggestionList } from "../hooks/useComposerSuggestionList.js";

describe("useComposerSuggestionList", () => {
  it("exports a hook function", () => {
    expect(typeof useComposerSuggestionList).toBe("function");
  });
});
