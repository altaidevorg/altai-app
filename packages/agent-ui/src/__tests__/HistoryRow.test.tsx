import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { HistoryRow } from "../components/HistoryRow.js";

describe("HistoryRow", () => {
  it("renders basename with full path title and detail", () => {
    const html = renderToStaticMarkup(
      createElement(HistoryRow, {
        path: "src/modules/ai/components/Foo.tsx",
        detail: "Applied 3 edits",
        restoring: false,
        onRestore: () => {},
      }),
    );
    expect(html).toContain("Foo.tsx");
    expect(html).toContain('title="src/modules/ai/components/Foo.tsx"');
    expect(html).toContain("Applied 3 edits");
    expect(html).toContain("Restore");
  });

  it("shows restoring state and disables button", () => {
    const html = renderToStaticMarkup(
      createElement(HistoryRow, {
        path: "README.md",
        detail: "Checkpoint 2",
        restoring: true,
        onRestore: () => {},
      }),
    );
    expect(html).toContain("Restoring…");
    expect(html).toContain("disabled");
    expect(html).toContain("disabled:opacity-40");
  });

  it("handles windows-style paths", () => {
    const html = renderToStaticMarkup(
      createElement(HistoryRow, {
        path: "src\\lib\\util.ts",
        detail: "Applied 1 edit",
        restoring: false,
        onRestore: () => {},
      }),
    );
    expect(html).toContain("util.ts");
  });
});
