import { createElement } from "react";
import { describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { TranscriptConversationEmpty } from "../components/TranscriptConversationEmpty.js";

describe("TranscriptConversationEmpty", () => {
  it("renders default title and description", () => {
    const html = renderToStaticMarkup(
      createElement(TranscriptConversationEmpty),
    );
    expect(html).toContain("Ask ALTAI anything");
    expect(html).toContain("Explain command output");
  });

  it("renders children slot", () => {
    const html = renderToStaticMarkup(
      createElement(
        TranscriptConversationEmpty,
        { title: "Empty" },
        createElement("span", null, "status-slot"),
      ),
    );
    expect(html).toContain("status-slot");
  });
});
