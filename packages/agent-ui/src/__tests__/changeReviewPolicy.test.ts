import { describe, expect, it } from "vitest";
import {
  dismissAllChangeReviewIds,
  dismissChangeReviewId,
  listChangeReviewItems,
  planLineDiffStats,
} from "../lib/changeReviewPolicy.js";

describe("listChangeReviewItems", () => {
  it("collects edit_diff tool rows and skips dismissed", () => {
    const items = listChangeReviewItems(
      [
        {
          id: "t1",
          role: "tool",
          filePath: "a.ts",
          diffOriginalText: "a",
          diffModifiedText: "b",
        },
        { id: "u1", role: "user" },
        {
          id: "t2",
          role: "tool",
          filePath: "b.ts",
          diffOriginalText: "",
          diffModifiedText: "new",
        },
      ],
      new Set(["t1"]),
    );
    expect(items).toHaveLength(1);
    expect(items[0]).toMatchObject({
      id: "t2",
      isNewFile: true,
      kind: "create_file",
    });
  });
});

describe("planLineDiffStats", () => {
  it("counts added and removed lines coarsely", () => {
    expect(planLineDiffStats("a\nb", "a\nc")).toEqual({
      added: 1,
      removed: 1,
    });
  });
});

describe("dismiss helpers", () => {
  it("tracks dismissed ids", () => {
    const one = dismissChangeReviewId(new Set(), "x");
    expect(one.has("x")).toBe(true);
    expect(
      dismissAllChangeReviewIds([{ id: "a" }, { id: "b" }]),
    ).toEqual(new Set(["a", "b"]));
  });
});
