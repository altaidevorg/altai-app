import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { WorkspaceTopbarActions } from "../components/WorkspaceTopbarActions.js";

const baseProps = {
  variant: "workspace" as const,
  workOpen: false,
  inboxOpen: false,
  inboxAttentionCount: 0,
  inspectorOpen: false,
  inspectorAvailable: true,
  onToggleWork: () => {},
  onToggleInbox: () => {},
  onToggleInspector: () => {},
};

describe("WorkspaceTopbarActions", () => {
  it("renders work, inbox, and run-details controls", () => {
    const html = renderToStaticMarkup(
      createElement(WorkspaceTopbarActions, baseProps),
    );
    expect(html).toContain("Open work");
    expect(html).toContain("Open inbox");
    expect(html).toContain("Open run details");
    expect(html).toContain("Work");
    expect(html).toContain("Inbox");
    expect(html).toContain("<svg");
  });

  it("shows attention badge and hides inspector when unavailable", () => {
    const html = renderToStaticMarkup(
      createElement(WorkspaceTopbarActions, {
        ...baseProps,
        variant: "sidebar",
        inboxAttentionCount: 120,
        inspectorAvailable: false,
        workOpen: true,
      }),
    );
    expect(html).toContain("99+");
    expect(html).toContain("120 need attention");
    expect(html).toContain("Close work");
    expect(html).not.toContain("Open run details");
    expect(html).not.toContain(">Work<");
  });
});
