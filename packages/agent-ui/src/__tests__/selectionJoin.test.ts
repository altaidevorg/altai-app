import { describe, expect, it } from "vitest";
import { joinSelectionTexts } from "../lib/selectionJoin.js";

describe("joinSelectionTexts", () => {
  it("returns null when nothing selected", () => {
    expect(
      joinSelectionTexts([
        {
          startLine: 0,
          startCharacter: 0,
          endLine: 0,
          endCharacter: 0,
          text: "",
        },
      ]),
    ).toBeNull();
  });

  it("passes through a single non-empty selection", () => {
    expect(
      joinSelectionTexts([
        {
          startLine: 1,
          startCharacter: 0,
          endLine: 1,
          endCharacter: 5,
          text: "hello",
        },
      ]),
    ).toEqual({
      text: "hello",
      range: {
        startLine: 1,
        startCharacter: 0,
        endLine: 1,
        endCharacter: 5,
      },
    });
  });

  it("labels and joins multi-cursor selections", () => {
    const joined = joinSelectionTexts([
      {
        startLine: 0,
        startCharacter: 0,
        endLine: 0,
        endCharacter: 1,
        text: "a",
      },
      {
        startLine: 4,
        startCharacter: 0,
        endLine: 4,
        endCharacter: 1,
        text: "b",
      },
    ]);
    expect(joined?.text).toContain("selection 1");
    expect(joined?.text).toContain("selection 2");
    expect(joined?.text).toContain("a");
    expect(joined?.text).toContain("b");
    expect(joined?.range).toEqual({
      startLine: 0,
      startCharacter: 0,
      endLine: 4,
      endCharacter: 1,
    });
  });
});
