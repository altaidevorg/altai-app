import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { ReviewHistory } from "../components/ReviewHistory.js";

describe("ReviewHistory", () => {
  it("returns null when there are no items", () => {
    const html = renderToStaticMarkup(
      createElement(ReviewHistory, {
        items: [],
        restoringId: null,
        error: null,
        onRestore: () => {},
      }),
    );
    expect(html).toBe("");
  });

  it("renders restore points and error", () => {
    const html = renderToStaticMarkup(
      createElement(ReviewHistory, {
        items: [
          {
            id: "a1",
            path: "src/a.ts",
            detail: "Accepted review · restore prior content",
          },
          {
            id: "c1",
            path: "src/b.ts",
            detail: "Before edit · 10:00 AM",
          },
        ],
        restoringId: "c1",
        error: "Could not restore change.",
        onRestore: () => {},
      }),
    );
    expect(html).toContain("Restore points");
    expect(html).toContain("pre-edit snapshot");
    expect(html).toContain("a.ts");
    expect(html).toContain("b.ts");
    expect(html).toContain("Accepted review");
    expect(html).toContain("Could not restore change.");
    expect(html).toContain("Restoring…");
  });
});
