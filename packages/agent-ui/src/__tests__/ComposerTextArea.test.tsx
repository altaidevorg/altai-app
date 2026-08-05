import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { ComposerTextArea } from "../components/ComposerTextArea.js";

describe("ComposerTextArea", () => {
  it("renders shared composer input defaults", () => {
    const html = renderToStaticMarkup(
      createElement(ComposerTextArea, {
        defaultValue: "Review these changes",
        placeholder: "Describe a task…",
      }),
    );

    expect(html).toContain("altai-ai-composer-textarea");
    expect(html).toContain('aria-label="Message ALTAI"');
    expect(html).toContain('rows="2"');
    expect(html).toContain("Review these changes");
    expect(html).toContain('placeholder="Describe a task…"');
  });

  it("accepts host accessibility and sizing overrides", () => {
    const html = renderToStaticMarkup(
      createElement(ComposerTextArea, {
        "aria-label": "Ask the agent",
        rows: 4,
        className: "custom-input",
      }),
    );

    expect(html).toContain('aria-label="Ask the agent"');
    expect(html).toContain('rows="4"');
    expect(html).toContain("custom-input");
  });
});
