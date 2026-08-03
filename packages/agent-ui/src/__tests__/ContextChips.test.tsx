import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { ContextChips, type ContextChip } from "../components/ContextChips.js";

describe("ContextChips", () => {
  it("renders nothing for an empty list", () => {
    const html = renderToStaticMarkup(
      createElement(ContextChips, { chips: [] }),
    );
    expect(html).toBe("");
  });

  it("renders selection, file, and snippet chips", () => {
    const chips: ContextChip[] = [
      { kind: "selection", source: "editor", lines: 3 },
      { kind: "file", name: "app.ts", lines: 12 },
      { kind: "snippet", name: "deploy" },
    ];
    const html = renderToStaticMarkup(
      createElement(ContextChips, { chips }),
    );
    expect(html).toContain("Editor selection");
    expect(html).toContain("· 3L");
    expect(html).toContain("app.ts");
    expect(html).toContain("· 12L");
    expect(html).toContain("#deploy");
  });
});
