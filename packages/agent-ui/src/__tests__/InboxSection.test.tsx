import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { InboxSection } from "../components/InboxSection.js";

describe("InboxSection", () => {
  it("renders section with header title and count", () => {
    const html = renderToStaticMarkup(
      createElement(
        InboxSection,
        { title: "Attention needed", count: 3 },
        createElement("div", null, "content"),
      ),
    );
    expect(html).toContain("<section");
    expect(html).toContain("Attention needed");
    expect(html).toContain("space-y-2");
  });

  it("renders children inside content area", () => {
    const html = renderToStaticMarkup(
      createElement(
        InboxSection,
        { title: "Waiting work", count: 0 },
        createElement("div", { "data-testid": "child" }, "job card"),
      ),
    );
    expect(html).toContain("job card");
  });
});
