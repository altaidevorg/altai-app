import { describe, expect, it, vi } from "vitest";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { AiSdkUiPartSwitch } from "../components/AiSdkUiPartSwitch.js";

describe("AiSdkUiPartSwitch", () => {
  it("routes text, reasoning, and tool to host slots", () => {
    const renderTool = vi.fn((p: { type?: string }) =>
      createElement("span", { "data-tool": p.type ?? "" }),
    );
    const html = renderToStaticMarkup(
      createElement(AiSdkUiPartSwitch, {
        part: { type: "text", text: "hello" },
        streaming: true,
        renderText: (text, streaming) =>
          createElement("p", { "data-stream": String(streaming) }, text),
        renderReasoning: (text) => createElement("aside", null, text),
        renderTool,
      }),
    );
    expect(html).toContain("hello");
    expect(html).toContain('data-stream="true"');

    const reason = renderToStaticMarkup(
      createElement(AiSdkUiPartSwitch, {
        part: { type: "reasoning", text: "think" },
        renderText: () => null,
        renderReasoning: (text) => createElement("aside", null, text),
        renderTool,
      }),
    );
    expect(reason).toContain("think");

    renderToStaticMarkup(
      createElement(AiSdkUiPartSwitch, {
        part: { type: "tool-read" },
        renderText: () => null,
        renderReasoning: () => null,
        renderTool,
      }),
    );
    expect(renderTool).toHaveBeenCalled();
  });
});
