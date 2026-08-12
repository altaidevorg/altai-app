import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { WorkspaceTopbarActions } from "../components/WorkspaceTopbarActions.js";

describe("WorkspaceTopbarActions", () => {
  it("renders the run-details control with a panel icon", () => {
    const html = renderToStaticMarkup(
      createElement(WorkspaceTopbarActions, {
        inspectorOpen: false,
        inspectorAvailable: true,
        onToggleInspector: () => {},
      }),
    );
    expect(html).toContain("Open details");
    expect(html).toContain(">Details<");
    expect(html).toContain("<svg");
    expect(html).not.toContain("Open work in Operations");
    expect(html).not.toContain("Open Operations inbox");
  });

  it("hides when unavailable and reflects open state", () => {
    const hidden = renderToStaticMarkup(
      createElement(WorkspaceTopbarActions, {
        inspectorOpen: false,
        inspectorAvailable: false,
        onToggleInspector: () => {},
      }),
    );
    expect(hidden).toBe("");

    const open = renderToStaticMarkup(
      createElement(WorkspaceTopbarActions, {
        inspectorOpen: true,
        inspectorAvailable: true,
        onToggleInspector: () => {},
        showLabel: false,
      }),
    );
    expect(open).toContain("Close details");
    expect(open).toContain('aria-pressed="true"');
    expect(open).not.toContain(">Details<");
  });
});
