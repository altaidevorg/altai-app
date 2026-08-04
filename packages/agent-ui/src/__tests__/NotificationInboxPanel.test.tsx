import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { NotificationInboxPanel } from "../components/NotificationInboxPanel.js";

const counts = { all: 0, attention: 0, updates: 0 };

describe("NotificationInboxPanel", () => {
  it("renders all-clear empty inbox", () => {
    const html = renderToStaticMarkup(
      createElement(NotificationInboxPanel, {
        attentionCount: 0,
        filter: "all",
        onFilterChange: () => {},
        filterCounts: counts,
        query: "",
        onQueryChange: () => {},
        empty: true,
        hasVisibleItems: false,
      }),
    );
    expect(html).toContain("Inbox");
    expect(html).toContain("All clear");
    expect(html).toContain("Nothing is blocking your agents");
  });

  it("renders attention banner and dismissible error chrome", () => {
    const html = renderToStaticMarkup(
      createElement(NotificationInboxPanel, {
        attentionCount: 2,
        filter: "all",
        onFilterChange: () => {},
        filterCounts: { all: 2, attention: 2, updates: 1 },
        query: "",
        onQueryChange: () => {},
        error: "Failed to load",
        onDismissError: () => {},
        empty: false,
        hasVisibleItems: false,
      }),
    );
    expect(html).toContain("Action needed");
    expect(html).toContain("Failed to load");
    expect(html).toContain("Dismiss error");
  });

  it("renders ticket and unread notification sections", () => {
    const html = renderToStaticMarkup(
      createElement(NotificationInboxPanel, {
        attentionCount: 2,
        filter: "all",
        onFilterChange: () => {},
        filterCounts: { all: 2, attention: 2, updates: 1 },
        query: "",
        onQueryChange: () => {},
        empty: false,
        hasVisibleItems: true,
        tickets: [
          {
            id: "t1",
            ticket: {
              prompt: "Which approach?",
              choices: ["A", "B"],
              updatedAtMs: 1,
            },
            canOpenChat: true,
            canResume: true,
            canDismiss: true,
          },
        ],
        unreadNotifications: [
          {
            id: "n1",
            notification: {
              title: "Run finished",
              body: "ok",
              kind: "run",
              createdAtMs: 1,
              seenAtMs: null,
            },
            canOpenChat: true,
          },
        ],
      }),
    );
    expect(html).toContain("Paused tasks");
    expect(html).toContain("Which approach?");
    expect(html).toContain("Unread updates");
    expect(html).toContain("Run finished");
  });
});
