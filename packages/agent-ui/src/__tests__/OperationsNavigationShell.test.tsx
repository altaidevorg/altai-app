import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { OperationsNavigationShell } from "../components/OperationsNavigationShell.js";

describe("OperationsNavigationShell", () => {
  it("renders only available Work OS routes", () => {
    const html = renderToStaticMarkup(
      createElement(
        OperationsNavigationShell,
        {
          view: "work",
          onViewChange: () => {},
          availableViews: ["work", "inbox"],
        },
        "body",
      ),
    );
    expect(html).toContain("Work navigation");
    expect(html).toContain("Work");
    expect(html).toContain("Inbox");
    expect(html).not.toContain("Overview");
    expect(html).not.toContain("Agents");
    expect(html).not.toContain("disabled");
  });

  it("marks the selected view", () => {
    const html = renderToStaticMarkup(
      createElement(
        OperationsNavigationShell,
        {
          view: "inbox",
          onViewChange: () => {},
          availableViews: ["work", "inbox"],
        },
        "inbox-body",
      ),
    );
    expect(html).toContain("inbox-body");
    expect(html).toContain('aria-selected="true"');
  });
});
