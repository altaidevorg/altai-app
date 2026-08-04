import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { InspectorSection } from "../components/InspectorSection.js";

describe("InspectorSection", () => {
  it("renders closed by default without children content", () => {
    const html = renderToStaticMarkup(
      createElement(
        InspectorSection,
        { title: "Todos", summary: "Open work", count: 3 },
        createElement("div", null, "todo body"),
      ),
    );
    expect(html).toContain("Todos");
    expect(html).toContain("Open work");
    expect(html).toContain(">3<");
    expect(html).toContain('aria-expanded="false"');
    expect(html).not.toContain("todo body");
    expect(html).toContain("<svg");
  });

  it("renders children when defaultOpen", () => {
    const html = renderToStaticMarkup(
      createElement(
        InspectorSection,
        {
          title: "Activity",
          summary: "Timeline",
          count: 0,
          defaultOpen: true,
        },
        createElement("div", null, "timeline body"),
      ),
    );
    expect(html).toContain('aria-expanded="true"');
    expect(html).toContain("timeline body");
    expect(html).not.toContain(">0<");
  });
});
