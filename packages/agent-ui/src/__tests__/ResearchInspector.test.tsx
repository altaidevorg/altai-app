import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { ResearchInspector } from "../components/ResearchInspector.js";

describe("ResearchInspector", () => {
  it("renders empty state when no events", () => {
    const html = renderToStaticMarkup(
      createElement(ResearchInspector, { events: [] }),
    );
    expect(html).toContain(
      "Web searches, fetched pages, and paper lookups will appear here.",
    );
  });

  it("renders events newest-first with label and detail", () => {
    const html = renderToStaticMarkup(
      createElement(ResearchInspector, {
        events: [
          {
            id: "1",
            label: "Older search",
            detail: "first",
            createdAt: 1_700_000_000_000,
          },
          {
            id: "2",
            label: "Newer search",
            detail: "second",
            createdAt: 1_700_000_100_000,
          },
        ],
      }),
    );
    expect(html).toContain("Newer search");
    expect(html).toContain("Older search");
    expect(html).toContain("second");
    expect(html.indexOf("Newer search")).toBeLessThan(html.indexOf("Older search"));
  });
});
