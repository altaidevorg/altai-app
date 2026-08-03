import { createElement } from "react";
import { describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { SelectionAskAi } from "../components/SelectionAskAi.js";

describe("SelectionAskAi", () => {
  it("renders Ask ALTAI with the host shortcut label", () => {
    const html = renderToStaticMarkup(
      createElement(SelectionAskAi, {
        x: 120,
        y: 80,
        onAsk: () => {},
        onDismiss: () => {},
        shortcutLabel: "⌘L",
        viewportWidth: 800,
      }),
    );
    expect(html).toContain("data-selection-ask-ai");
    expect(html).toContain("Ask ALTAI");
    expect(html).toContain("⌘L");
  });

  it("clamps horizontal position within the viewport", () => {
    const html = renderToStaticMarkup(
      createElement(SelectionAskAi, {
        x: 10,
        y: 40,
        onAsk: vi.fn(),
        onDismiss: vi.fn(),
        shortcutLabel: "Ctrl+L",
        viewportWidth: 200,
      }),
    );
    expect(html).toContain("left: 8px");
    expect(html).toContain("width: 110px");
  });
});
