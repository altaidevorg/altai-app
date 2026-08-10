import { describe, expect, it, vi } from "vitest";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { AiDisplayMessageContent } from "../components/AiDisplayMessageContent.js";

describe("AiDisplayMessageContent", () => {
  it("renders text and stream marker", () => {
    const html = renderToStaticMarkup(
      createElement(AiDisplayMessageContent, {
        content: "hello",
        streaming: true,
      }),
    );
    expect(html).toContain("hello");
    expect(html).toContain("▍");
    expect(html).toContain("altai-chat-bubble-body");
  });

  it("renders static path when cannot open", () => {
    const html = renderToStaticMarkup(
      createElement(AiDisplayMessageContent, {
        content: "see /Users/me/proj/src/app.ts for details",
        canOpenFile: false,
      }),
    );
    expect(html).toContain("altai-chat-path");
  });

  it("renders GFM text through the shared Markdown renderer", () => {
    const html = renderToStaticMarkup(
      createElement(AiDisplayMessageContent, {
        content: "## Status\n\n| Item | State |\n| --- | --- |\n| UI | Ready |",
      }),
    );

    expect(html).toContain("Status");
    expect(html).toContain("<table");
  });

  it("invokes callbacks via host open flags only when enabled", () => {
    const onOpenPath = vi.fn();
    renderToStaticMarkup(
      createElement(AiDisplayMessageContent, {
        content: "plain",
        canOpenFile: true,
        onOpenPath,
      }),
    );
    expect(onOpenPath).not.toHaveBeenCalled();
  });
});
