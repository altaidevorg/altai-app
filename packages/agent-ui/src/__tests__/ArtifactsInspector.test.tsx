import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { ArtifactsInspector } from "../components/ArtifactsInspector.js";

describe("ArtifactsInspector", () => {
  it("renders empty state when no items", () => {
    const html = renderToStaticMarkup(
      createElement(ArtifactsInspector, { items: [], onOpenFile: () => {} }),
    );
    expect(html).toContain(
      "Files emitted by experiments and execution jobs will appear here.",
    );
  });

  it("renders basename, full path, and Open button newest-first", () => {
    const html = renderToStaticMarkup(
      createElement(ArtifactsInspector, {
        items: [
          { id: "1", path: "out/older.txt" },
          { id: "2", path: "out/newer.txt" },
        ],
        onOpenFile: () => {},
      }),
    );
    expect(html).toContain("newer.txt");
    expect(html).toContain("older.txt");
    expect(html).toContain("out/newer.txt");
    expect(html).toContain("Open");
    expect(html.indexOf("newer.txt")).toBeLessThan(html.indexOf("older.txt"));
  });
});
