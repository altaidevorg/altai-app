import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { AiRunInspectorFrame } from "../components/AiRunInspectorFrame.js";

describe("AiRunInspectorFrame", () => {
  it("renders the Desktop sidebar frame with a labelled inspector landmark", () => {
    const html = renderToStaticMarkup(
      createElement(AiRunInspectorFrame, {
        header: createElement("header", null, "Header"),
        summary: createElement("div", null, "Summary"),
        children: createElement("div", null, "Sections"),
      }),
    );

    expect(html).toContain("data-ai-run-inspector-frame");
    expect(html).toContain('data-variant="sidebar"');
    expect(html).toContain('aria-label="Details"');
    expect(html).toContain("Summary");
    expect(html).toContain("Sections");
  });

  it("uses a section for compact host chrome", () => {
    const html = renderToStaticMarkup(
      createElement(AiRunInspectorFrame, {
        variant: "compact",
        header: "Header",
        summary: "Summary",
      }),
    );

    expect(html).toMatch(/^<section/);
    expect(html).toContain('data-variant="compact"');
  });
});
