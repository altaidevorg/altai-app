import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { AiPanelTopbar } from "../components/AiPanelTopbar.js";

describe("AiPanelTopbar", () => {
  it("provides one labelled shared chrome frame for host rows", () => {
    const html = renderToStaticMarkup(
      createElement(AiPanelTopbar, {
        primary: createElement("div", null, "Primary"),
        secondary: createElement("div", null, "Secondary"),
        "aria-label": "ALTAI panel chrome",
      }),
    );

    expect(html).toContain("data-ai-panel-topbar");
    expect(html).toContain('aria-label="ALTAI panel chrome"');
    expect(html).toContain("Primary");
    expect(html).toContain("Secondary");
  });
});
