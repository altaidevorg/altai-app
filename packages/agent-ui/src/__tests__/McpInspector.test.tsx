import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { McpInspector } from "../components/McpInspector.js";

describe("McpInspector", () => {
  it("renders empty state when no events", () => {
    const html = renderToStaticMarkup(
      createElement(McpInspector, { events: [] }),
    );
    expect(html).toContain(
      "MCP server calls will appear here when the agent uses a connected tool.",
    );
  });

  it("renders tone-colored events newest-first", () => {
    const html = renderToStaticMarkup(
      createElement(McpInspector, {
        events: [
          {
            id: "1",
            label: "Older call",
            tone: "success",
            createdAt: 1_700_000_000_000,
          },
          {
            id: "2",
            label: "Failed call",
            tone: "error",
            detail: "timeout",
            createdAt: 1_700_000_100_000,
          },
        ],
      }),
    );
    expect(html).toContain("Failed call");
    expect(html).toContain("Older call");
    expect(html).toContain("timeout");
    expect(html).toContain("bg-destructive");
    expect(html).toContain("bg-foreground");
    expect(html.indexOf("Failed call")).toBeLessThan(html.indexOf("Older call"));
  });
});
