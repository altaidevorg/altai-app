import { describe, expect, it } from "vitest";
import {
  nextSuggestionActiveIndex,
  resolveComposerSuggestionKeyAction,
  resolveComposerSuggestionOpen,
} from "../lib/composerSuggestionKeyboard.js";

describe("composerSuggestionKeyboard", () => {
  it("clamps active index", () => {
    expect(nextSuggestionActiveIndex(0, 3, "down")).toBe(1);
    expect(nextSuggestionActiveIndex(2, 3, "down")).toBe(2);
    expect(nextSuggestionActiveIndex(1, 3, "up")).toBe(0);
    expect(nextSuggestionActiveIndex(0, 0, "down")).toBe(0);
  });

  it("resolves open state for prefix", () => {
    expect(
      resolveComposerSuggestionOpen({
        trigger: { prefix: "/", query: "ta" },
        forceClosed: false,
        prefix: "/",
      }),
    ).toEqual({ open: true, query: "ta" });
    expect(
      resolveComposerSuggestionOpen({
        trigger: { prefix: "/", query: "ta" },
        forceClosed: false,
        prefix: "#",
      }),
    ).toEqual({ open: false, query: "" });
  });

  it("maps keys to close/move/pick/ignore", () => {
    expect(
      resolveComposerSuggestionKeyAction("Escape", {
        matchCount: 2,
        activeIndex: 0,
      }),
    ).toEqual({ type: "close" });
    expect(
      resolveComposerSuggestionKeyAction("Enter", {
        matchCount: 0,
        activeIndex: 0,
      }),
    ).toEqual({ type: "ignore" });
    expect(
      resolveComposerSuggestionKeyAction("ArrowDown", {
        matchCount: 3,
        activeIndex: 0,
      }),
    ).toEqual({ type: "move", index: 1 });
    expect(
      resolveComposerSuggestionKeyAction("Enter", {
        matchCount: 3,
        activeIndex: 1,
      }),
    ).toEqual({ type: "pick", index: 1 });
  });
});
