import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { AiSidePanelFrame } from "../components/AiSidePanelFrame.js";

describe("AiSidePanelFrame", () => {
  it("renders the shared panel landmark with topbar and body", () => {
    const html = renderToStaticMarkup(
      createElement(AiSidePanelFrame, {
        variant: "workspace",
        topbar: createElement("header", null, "Topbar"),
        children: createElement("main", null, "Body"),
      }),
    );
    expect(html).toContain("altai-ai-panel");
    expect(html).toContain('data-ai-workspace="true"');
    expect(html).toContain("Topbar");
    expect(html).toContain("Body");
    expect(html).toContain('aria-label="ALTAI agent workspace"');
  });

  it("defaults to sidebar assistant label", () => {
    const html = renderToStaticMarkup(
      createElement(AiSidePanelFrame, {
        topbar: null,
        children: "Chat",
      }),
    );
    expect(html).toContain('aria-label="AI assistant"');
    expect(html).not.toContain("data-ai-workspace");
  });
});
