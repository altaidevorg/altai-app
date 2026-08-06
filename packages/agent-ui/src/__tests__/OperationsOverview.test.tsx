import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { OperationsOverview } from "../components/OperationsOverview.js";

describe("OperationsOverview", () => {
  it("renders metrics, attention rows, and progressing rows", () => {
    const html = renderToStaticMarkup(
      createElement(OperationsOverview, {
        status: "ready",
        metrics: [
          { label: "Active runs", value: "2" },
          { label: "Unread inbox", value: "3" },
        ],
        attention: [
          {
            id: "n1",
            title: "Approval requested",
            statusLabel: "Needs approval",
            tone: "attention",
            onOpen: () => {},
          },
        ],
        progressing: [
          {
            id: "t1",
            title: "Refactor parser",
            statusLabel: "Working",
            detail: "step 3",
            actions: createElement("button", { type: "button" }, "Stop"),
          },
        ],
      }),
    );
    expect(html).toContain("Operations overview");
    expect(html).toContain("Active runs");
    expect(html).toContain("Needs attention");
    expect(html).toContain("Approval requested");
    expect(html).toContain("In progress");
    expect(html).toContain("Refactor parser");
    expect(html).toContain("Stop");
    // Row with onOpen renders as a button.
    expect(html).toContain("<button");
  });

  it("renders empty labels when sections have no rows", () => {
    const html = renderToStaticMarkup(
      createElement(OperationsOverview, { status: "ready" }),
    );
    expect(html).toContain("Nothing needs attention.");
    expect(html).toContain("No active work right now.");
  });

  it("renders a loading state", () => {
    const html = renderToStaticMarkup(
      createElement(OperationsOverview, { status: "loading" }),
    );
    expect(html).toContain("Loading operations…");
  });

  it("renders an inline error with dismiss action", () => {
    const html = renderToStaticMarkup(
      createElement(OperationsOverview, {
        status: "error",
        errorMessage: "host_not_ready",
        onDismissError: () => {},
      }),
    );
    expect(html).toContain("host_not_ready");
    expect(html).toContain("Dismiss");
  });
});
