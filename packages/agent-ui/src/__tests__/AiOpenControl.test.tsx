import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { AiOpenControl } from "../components/AiOpenControl.js";

describe("AiOpenControl", () => {
  it("renders inactive toggle", () => {
    const html = renderToStaticMarkup(
      createElement(AiOpenControl, {
        onOpen: () => {},
        title: "Show AI agent  ⌘I",
      }),
    );
    expect(html).toContain("Show AI agent");
    expect(html).toContain("⌘I");
    expect(html).toContain('aria-pressed="false"');
    expect(html).toContain("<svg");
  });

  it("marks active state", () => {
    const html = renderToStaticMarkup(
      createElement(AiOpenControl, {
        active: true,
        onOpen: () => {},
        title: "Hide AI agent",
      }),
    );
    expect(html).toContain("Hide AI agent");
    expect(html).toContain('aria-pressed="true"');
  });
});
