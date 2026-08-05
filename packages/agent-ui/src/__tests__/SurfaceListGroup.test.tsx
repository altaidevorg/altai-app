import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { SurfaceListGroup } from "../components/SurfaceListGroup.js";

describe("SurfaceListGroup", () => {
  it("renders a titled card group with count and description", () => {
    const html = renderToStaticMarkup(
      createElement(
        SurfaceListGroup,
        {
          title: "Needs attention",
          description: "Runs waiting on you",
          count: 2,
          className: "space-group",
          containerClassName: "custom-list",
          children: createElement("article", null, "Fix the build"),
        },
      ),
    );

    expect(html).toContain("<section class=\"space-group\"");
    expect(html).toContain("Needs attention");
    expect(html).toContain("Runs waiting on you");
    expect(html).toContain(">2<");
    expect(html).toContain("custom-list");
    expect(html).toContain("Fix the build");
  });

  it("supports semantic list rows", () => {
    const html = renderToStaticMarkup(
      createElement(
        SurfaceListGroup,
        {
          title: "Workspace schedules",
          containerAs: "ul",
          containerAriaLabel: "Workspace automations",
          children: createElement("li", null, "Review changes"),
        },
      ),
    );

    expect(html).toContain("<ul aria-label=\"Workspace automations\"");
    expect(html).toContain("<li>Review changes</li>");
  });
});
