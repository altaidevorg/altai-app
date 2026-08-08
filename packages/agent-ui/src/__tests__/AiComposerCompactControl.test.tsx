import { describe, expect, it, vi } from "vitest";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { AiComposerCompactControl } from "../components/AiComposerCompactControl.js";

describe("AiComposerCompactControl", () => {
  it("returns null without capability or chat", () => {
    const html = renderToStaticMarkup(
      createElement(AiComposerCompactControl, {
        canCompact: false,
        hasActiveChat: true,
        onCompact: vi.fn(),
      }),
    );
    expect(html).toBe("");
  });

  it("renders when allowed", () => {
    const html = renderToStaticMarkup(
      createElement(AiComposerCompactControl, {
        canCompact: true,
        hasActiveChat: true,
        onCompact: vi.fn(),
      }),
    );
    expect(html.length).toBeGreaterThan(0);
  });
});
