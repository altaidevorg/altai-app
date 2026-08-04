import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { SurfaceFilterToolbar } from "../components/SurfaceFilterToolbar.js";

describe("SurfaceFilterToolbar", () => {
  it("renders search and filter tabs", () => {
    const html = renderToStaticMarkup(
      createElement(SurfaceFilterToolbar, {
        query: "auth",
        onQueryChange: () => {},
        searchPlaceholder: "Search by task, step, or result",
        tabsLabel: "Filter work runs",
        tabValue: "active",
        onTabChange: () => {},
        tabs: [
          { id: "all", label: "All", count: 3 },
          { id: "active", label: "Live", count: 1 },
        ],
      }),
    );
    expect(html).toContain("Search by task, step, or result");
    expect(html).toContain("Filter work runs");
    expect(html).toContain("Live");
    expect(html).toContain('value="auth"');
  });
});
