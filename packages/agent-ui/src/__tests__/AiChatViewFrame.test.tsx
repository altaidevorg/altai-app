/** @jsxImportSource react */
import { describe, expect, it } from "vitest";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { AiChatViewFrame } from "../components/AiChatViewFrame.js";

describe("AiChatViewFrame", () => {
  it("renders empty frame with status slot", () => {
    const html = renderToStaticMarkup(
      createElement(AiChatViewFrame, {
        messages: [],
        status: "idle",
        announce: "polite",
        emptyStatus: createElement("span", { "data-empty": "1" }),
        renderMessage: () => null,
        renderRoot: ({ body, "aria-live": live }) =>
          createElement("div", { "data-root": "1", "aria-live": live }, body),
      }),
    );
    expect(html).toContain('data-root="1"');
    expect(html).toContain('data-empty="1"');
    expect(html).toContain("altai-ai-transcript-empty");
  });

  it("maps messages through renderMessage", () => {
    const html = renderToStaticMarkup(
      createElement(AiChatViewFrame, {
        messages: [
          { id: "u1", role: "user" },
          { id: "a1", role: "assistant" },
        ],
        status: "idle",
        renderMessage: ({ message, streaming, canRetry }) =>
          createElement(
            "article",
            {
              key: message.id,
              "data-id": message.id,
              "data-streaming": String(streaming),
              "data-retry": String(canRetry),
            },
            message.role,
          ),
        renderRoot: ({ body }) => createElement("div", null, body),
      }),
    );
    expect(html).toContain('data-id="u1"');
    expect(html).toContain('data-id="a1"');
  });
});
