import { describe, expect, it, vi } from "vitest";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { AiComposerFollowupControl } from "../components/AiComposerFollowupControl.js";

describe("AiComposerFollowupControl", () => {
  it("returns null without active run", () => {
    const html = renderToStaticMarkup(
      createElement(AiComposerFollowupControl, {
        hasActiveRun: false,
        hasPrompt: true,
        canSteer: true,
        canQueue: true,
        onSteer: vi.fn(),
        onQueue: vi.fn(),
      }),
    );
    expect(html).toBe("");
  });

  it("renders when run and capability allow", () => {
    const html = renderToStaticMarkup(
      createElement(AiComposerFollowupControl, {
        hasActiveRun: true,
        hasPrompt: true,
        canSteer: true,
        canQueue: false,
        onSteer: vi.fn(),
        onQueue: vi.fn(),
      }),
    );
    expect(html.length).toBeGreaterThan(0);
  });
});
