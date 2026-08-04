import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { UnifiedDiffPreview } from "../components/UnifiedDiffPreview.js";

describe("UnifiedDiffPreview", () => {
  it("renders no changes message when original equals proposed", () => {
    const html = renderToStaticMarkup(
      createElement(UnifiedDiffPreview, {
        original: "hello\nworld",
        proposed: "hello\nworld",
      }),
    );
    expect(html).toContain("no line-level changes");
  });

  it("renders added and removed lines", () => {
    const html = renderToStaticMarkup(
      createElement(UnifiedDiffPreview, {
        original: "line1\nline2\nline3",
        proposed: "line1\nline2-modified\nline3",
      }),
    );
    expect(html).toContain("line2");
    expect(html).toContain("line2-modified");
    expect(html).toContain("text-success");
    expect(html).toContain("text-destructive");
  });

  it("truncates after 80 lines", () => {
    const original = Array.from({ length: 100 }, (_, i) => `old${i}`).join("\n");
    const proposed = Array.from({ length: 100 }, (_, i) => `new${i}`).join("\n");
    const html = renderToStaticMarkup(
      createElement(UnifiedDiffPreview, { original, proposed }),
    );
    expect(html).toContain("more changes");
  });
});
