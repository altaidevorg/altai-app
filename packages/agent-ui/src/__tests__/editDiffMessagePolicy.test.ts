import { describe, expect, it } from "vitest";
import {
  countPendingEditDiffs,
  isEditDiffMessage,
  lastEditDiffMessage,
} from "../lib/editDiffMessagePolicy.js";

describe("editDiffMessagePolicy", () => {
  const msgs = [
    { role: "user" },
    {
      role: "tool",
      diffOriginalText: "a",
      diffModifiedText: "b",
      id: "d1",
    },
    { role: "assistant" },
    {
      role: "tool",
      diffOriginalText: "c",
      diffModifiedText: "d",
      id: "d2",
    },
  ];

  it("detects edit-diff rows", () => {
    expect(isEditDiffMessage(msgs[1]!)).toBe(true);
    expect(isEditDiffMessage(msgs[0]!)).toBe(false);
    expect(countPendingEditDiffs(msgs)).toBe(2);
    expect(lastEditDiffMessage(msgs)?.id).toBe("d2");
  });
});
