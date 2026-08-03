import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { FilteredEmptyInbox } from "../components/FilteredEmptyInbox.js";

describe("FilteredEmptyInbox", () => {
  it("renders label and show-all button", () => {
    const html = renderToStaticMarkup(
      createElement(FilteredEmptyInbox, {
        label: "Nothing needs your attention",
        onShowAll: () => {},
      }),
    );
    expect(html).toContain("Nothing needs your attention");
    expect(html).toContain("Show all inbox items");
    expect(html).toContain("<svg");
  });

  it("renders alternative label", () => {
    const html = renderToStaticMarkup(
      createElement(FilteredEmptyInbox, {
        label: "No updates to show",
        onShowAll: () => {},
      }),
    );
    expect(html).toContain("No updates to show");
  });
});
