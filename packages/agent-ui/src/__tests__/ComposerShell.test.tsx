import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { ComposerShell } from "../components/ComposerShell.js";

describe("ComposerShell", () => {
  it("renders the shared surface with attachment chrome", () => {
    const html = renderToStaticMarkup(
      createElement(ComposerShell, {
        busy: true,
        attachments: createElement("span", null, "readme.md"),
        children: createElement("textarea", { "aria-label": "Message" }),
      }),
    );

    expect(html).toContain("altai-ai-composer");
    expect(html).toContain("altai-ai-composer-attachments");
    expect(html).toContain("opacity-95");
    expect(html).toContain("readme.md");
    expect(html).toContain('aria-label="Message"');
  });

  it("omits the attachment divider when the slot is empty", () => {
    const html = renderToStaticMarkup(
      createElement(ComposerShell, {
        children: "Composer body",
        className: "custom-shell",
      }),
    );

    expect(html).not.toContain("altai-ai-composer-attachments");
    expect(html).toContain("custom-shell");
    expect(html).toContain("Composer body");
  });
});
