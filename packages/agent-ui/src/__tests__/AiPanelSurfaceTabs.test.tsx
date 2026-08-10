import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { AiPanelSurfaceTabs } from "../components/AiPanelSurfaceTabs.js";

describe("AiPanelSurfaceTabs", () => {
  it("renders a labelled tablist and marks only the active surface selected", () => {
    const html = renderToStaticMarkup(
      createElement(AiPanelSurfaceTabs, {
        activeId: "chat",
        tabs: [
          { id: "chat", label: "Chat" },
          { id: "operations", label: "Operations" },
        ],
        onSelect: () => undefined,
        "aria-label": "ALTAI surfaces",
      }),
    );

    expect(html).toContain("data-ai-panel-surface-tabs");
    expect(html).toContain('role="tablist"');
    expect(html).toContain('aria-label="ALTAI surfaces"');
    expect(html).toContain('aria-selected="true"');
    expect(html).toContain('aria-selected="false"');
  });
});
