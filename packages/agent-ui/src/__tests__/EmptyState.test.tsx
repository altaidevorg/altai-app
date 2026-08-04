import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { EmptyState } from "../components/EmptyState.js";

describe("EmptyState", () => {
  it("renders agent name and home copy", () => {
    const html = renderToStaticMarkup(
      createElement(EmptyState, { agentName: "Coder" }),
    );
    expect(html).toContain("Coder · ready");
    expect(html).toContain("Start with the outcome");
    expect(html).toContain("Open IDE");
    expect(html).toContain("<svg");
  });
});
