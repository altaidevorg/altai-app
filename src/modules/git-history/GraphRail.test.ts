import { describe, expect, it } from "vitest";
import {
  isVisibleGraphEdge,
  MAX_VISIBLE_LANES,
  railWidth,
} from "./GraphRail";

describe("commit graph rail", () => {
  it("keeps the reserved rail width independent of deep hidden lanes", () => {
    expect(railWidth(MAX_VISIBLE_LANES)).toBe(108);
    expect(railWidth(MAX_VISIBLE_LANES + 20)).toBe(108);
  });

  it("never renders an edge when either end lies beyond the visible rail", () => {
    expect(
      isVisibleGraphEdge(
        { kind: "straight", lane: MAX_VISIBLE_LANES, color: "#000" },
        MAX_VISIBLE_LANES,
      ),
    ).toBe(false);
    expect(
      isVisibleGraphEdge(
        {
          kind: "branch",
          fromLane: MAX_VISIBLE_LANES - 1,
          toLane: MAX_VISIBLE_LANES,
          color: "#000",
        },
        MAX_VISIBLE_LANES,
      ),
    ).toBe(false);
    expect(
      isVisibleGraphEdge(
        { kind: "merge", fromLane: 1, toLane: 0, color: "#000" },
        MAX_VISIBLE_LANES,
      ),
    ).toBe(true);
  });
});
