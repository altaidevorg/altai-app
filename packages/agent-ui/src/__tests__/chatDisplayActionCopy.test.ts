import { describe, expect, it } from "vitest";
import {
  displayCopyActionLabel,
  displayDiffReviewTitle,
  displayOpenDiffActionTitle,
  displayOpenFileActionTitle,
  displayOpeningActionLabel,
} from "../lib/chatDisplayActionCopy.js";

describe("chatDisplayActionCopy", () => {
  it("builds action labels and titles", () => {
    expect(displayCopyActionLabel(false)).toBe("Copy");
    expect(displayCopyActionLabel(true)).toBe("Copied");
    expect(displayOpenFileActionTitle("a.ts")).toBe("Open a.ts");
    expect(displayOpenFileActionTitle()).toBe("Open file");
    expect(displayOpenDiffActionTitle("b.ts")).toBe("Review diff for b.ts");
    expect(displayOpenDiffActionTitle()).toBe("Open diff");
    expect(displayOpeningActionLabel(true, "Diff")).toBe("Opening…");
    expect(displayOpeningActionLabel(false, "Diff")).toBe("Diff");
    expect(displayDiffReviewTitle("c.ts")).toBe("ALTAI · c.ts");
    expect(displayDiffReviewTitle()).toBe("ALTAI review");
  });
});
