import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import {
  AutomationList,
  type AutomationListItem,
} from "../components/AutomationList.js";

function item(id: string, message: string): AutomationListItem {
  return {
    id,
    message,
    scheduleLabel: "Every 1h",
    nextRunLabel: "Next soon",
    lastRunLabel: "Not run yet",
    owningChatLabel: "Ops chat",
    onOpenChat: () => {},
    onDuplicate: () => {},
    onRemove: () => {},
  };
}

describe("AutomationList", () => {
  it("renders scheduled rows in shared list chrome", () => {
    const html = renderToStaticMarkup(
      createElement(AutomationList, {
        items: [item("first", "Review changes"), item("second", "Run tests")],
      }),
    );

    expect(html).toContain("Workspace schedules");
    expect(html).toContain("Ordered by the next expected run");
    expect(html).toContain('aria-label="Workspace automations"');
    expect(html).toContain("Review changes");
    expect(html).toContain("Run tests");
    expect(html).toContain("border-t border-border-subtle");
  });

  it("accepts host-provided list copy", () => {
    const html = renderToStaticMarkup(
      createElement(AutomationList, {
        items: [item("one", "Health check")],
        title: "Project schedules",
        description: "Ordered by priority",
        ariaLabel: "Project automations",
      }),
    );

    expect(html).toContain("Project schedules");
    expect(html).toContain("Ordered by priority");
    expect(html).toContain('aria-label="Project automations"');
  });
});
