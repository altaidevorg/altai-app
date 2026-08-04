import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { ChangesInspector } from "../components/ChangesInspector.js";

describe("ChangesInspector", () => {
  it("renders empty state when queue is empty", () => {
    const html = renderToStaticMarkup(
      createElement(ChangesInspector, { queue: [], onOpenReview: () => {} }),
    );
    expect(html).toContain(
      "Planned and agent-made changes will appear here for review.",
    );
  });

  it("renders summary, open button, and change rows", () => {
    const html = renderToStaticMarkup(
      createElement(ChangesInspector, {
        queue: [
          {
            id: "1",
            path: "src/a.ts",
            originalContent: "a\nb",
            proposedContent: "a\nb\nc",
            isNewFile: false,
          },
          {
            id: "2",
            path: "src/b.ts",
            originalContent: "",
            proposedContent: "new",
            isNewFile: true,
          },
        ],
        onOpenReview: () => {},
      }),
    );
    expect(html).toContain("2 proposed changes are waiting for review.");
    expect(html).toContain("Open change review");
    expect(html).toContain("a.ts");
    expect(html).toContain("+1L");
    expect(html).toContain("new");
  });

  it("uses singular copy for one change", () => {
    const html = renderToStaticMarkup(
      createElement(ChangesInspector, {
        queue: [
          {
            id: "1",
            path: "only.ts",
            originalContent: "x",
            proposedContent: "y",
            isNewFile: false,
          },
        ],
        onOpenReview: () => {},
      }),
    );
    expect(html).toContain("1 proposed change is waiting for review.");
  });
});
