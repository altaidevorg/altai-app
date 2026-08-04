import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { ActivityInspector } from "../components/ActivityInspector.js";

describe("ActivityInspector", () => {
  it("renders compact timeline and empty search copy", () => {
    const html = renderToStaticMarkup(
      createElement(ActivityInspector, {
        events: [],
        hasQuery: true,
        compact: true,
      }),
    );
    expect(html).toContain("Timeline");
    expect(html).toContain("No timeline events match this search.");
    expect(html).not.toContain("Run state");
  });

  it("renders newest events first with tone and detail", () => {
    const html = renderToStaticMarkup(
      createElement(ActivityInspector, {
        compact: true,
        hasQuery: false,
        events: [
          {
            id: "1",
            label: "Older",
            createdAt: 1_700_000_000_000,
            tone: "default",
          },
          {
            id: "2",
            label: "Newer",
            detail: "done",
            createdAt: 1_700_000_060_000,
            tone: "success",
          },
        ],
      }),
    );
    expect(html.indexOf("Newer")).toBeLessThan(html.indexOf("Older"));
    expect(html).toContain("done");
    expect(html).toContain("bg-success");
  });

  it("renders non-compact run state when requested", () => {
    const html = renderToStaticMarkup(
      createElement(ActivityInspector, {
        compact: false,
        hasQuery: false,
        events: [],
        statusPill: createElement("span", null, "status"),
        step: "Reading files",
        error: "Something failed",
        approvalsPending: 2,
        subagentCount: 1,
        inputTokens: 100,
        outputTokens: 50,
      }),
    );
    expect(html).toContain("status");
    expect(html).toContain("150 tokens");
    expect(html).toContain("Reading files");
    expect(html).toContain("Run state");
    expect(html).toContain("Approvals");
    expect(html).toContain("Something failed");
    expect(html).toContain("Run events will appear here");
  });
});
