import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { AssistantBrandLabel } from "../components/AssistantBrandLabel.js";

describe("AssistantBrandLabel", () => {
  it("renders brand without streaming hint", () => {
    const html = renderToStaticMarkup(createElement(AssistantBrandLabel));
    expect(html).toContain("ALTAI");
    expect(html).not.toContain("thinking");
  });

  it("renders streaming hint when active", () => {
    const html = renderToStaticMarkup(
      createElement(AssistantBrandLabel, {
        streaming: true,
        streamingLabel: "working…",
      }),
    );
    expect(html).toContain("working…");
  });
});
