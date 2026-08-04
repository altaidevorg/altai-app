import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { EmptyInbox } from "../components/EmptyInbox.js";

describe("EmptyInbox", () => {
  it("renders the all-caught-up message", () => {
    const html = renderToStaticMarkup(createElement(EmptyInbox));
    expect(html).toContain("all caught up");
    expect(html).toContain(
      "Questions, review-ready results, and durable agent updates will appear here.",
    );
    expect(html).toContain("<svg");
    expect(html).toContain("border-0 bg-transparent");
  });
});
