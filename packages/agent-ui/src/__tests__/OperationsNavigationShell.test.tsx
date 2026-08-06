import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { OperationsNavigationShell } from "../components/OperationsNavigationShell.js";

describe("OperationsNavigationShell", () => {
  it("renders shared routes and disables unavailable domain slices", () => {
    const html = renderToStaticMarkup(
      createElement(
        OperationsNavigationShell,
        {
          view: "overview",
          onViewChange: () => {},
          availableViews: ["overview"],
        },
        "body",
      ),
    );
    expect(html).toContain("Operations navigation");
    expect(html).toContain("Overview");
    expect(html).toContain("Work");
    expect(html).toContain('disabled=""');
  });

  it("marks enabled routes as interactive and selected view as selected", () => {
    const html = renderToStaticMarkup(
      createElement(
        OperationsNavigationShell,
        {
          view: "runs",
          onViewChange: () => {},
          availableViews: ["overview", "work", "runs", "inbox"],
        },
        "runs-body",
      ),
    );
    expect(html).toContain("runs-body");
    expect(html).toContain('aria-selected="true"');
    // Overview is available but not selected — no disabled attrs on those four tabs.
    // Agents remains disabled (not in availableViews).
    expect(html).toContain("Agents");
    expect(html).toMatch(/Agents[\s\S]*disabled/);
  });
});
