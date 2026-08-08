import { describe, expect, it } from "vitest";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { AiAssistantRunActions } from "../components/AiAssistantRunActions.js";

describe("AiAssistantRunActions", () => {
  it("renders stop while streaming", () => {
    const html = renderToStaticMarkup(
      createElement(AiAssistantRunActions, {
        streaming: true,
        renderStop: () => createElement("button", null, "Stop"),
        renderRetry: () => createElement("button", null, "Retry"),
      }),
    );
    expect(html).toContain("Stop");
    expect(html).not.toContain("Retry");
  });

  it("renders null when hidden", () => {
    const html = renderToStaticMarkup(
      createElement(AiAssistantRunActions, {
        streaming: false,
        renderStop: () => createElement("button", null, "Stop"),
        renderRetry: () => createElement("button", null, "Retry"),
      }),
    );
    expect(html).toBe("");
  });
});
