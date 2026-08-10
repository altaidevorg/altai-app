import { describe, expect, it } from "vitest";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { AiDisplayMessageBodyExtras } from "../components/AiDisplayMessageBodyExtras.js";

describe("AiDisplayMessageBodyExtras", () => {
  it("renders children and optional diff + todos wrappers", () => {
    const html = renderToStaticMarkup(
      createElement(AiDisplayMessageBodyExtras, {
        children: createElement("p", null, "main"),
        originalText: "a",
        proposedText: "b",
        todos: [{ id: "1", title: "t", status: "pending" }],
      }),
    );
    expect(html).toContain("main");
    expect(html).toContain("altai-chat-inline-diff");
    expect(html).toContain("altai-chat-todos");
  });

  it("skips extras when absent", () => {
    const html = renderToStaticMarkup(
      createElement(AiDisplayMessageBodyExtras, {
        children: createElement("p", null, "only"),
      }),
    );
    expect(html).toContain("only");
    expect(html).not.toContain("altai-chat-inline-diff");
    expect(html).not.toContain("altai-chat-todos");
  });
});
