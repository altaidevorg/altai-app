import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { HoverActionButton } from "../components/HoverActionButton.js";

describe("HoverActionButton", () => {
  it("renders a button with title, aria-label, and children", () => {
    const html = renderToStaticMarkup(
      createElement(
        HoverActionButton,
        { title: "Stop generating", onClick: () => {} },
        createElement("span", { "data-icon": "stop" }, "■"),
      ),
    );
    expect(html).toContain('type="button"');
    expect(html).toContain('title="Stop generating"');
    expect(html).toContain('aria-label="Stop generating"');
    expect(html).toContain('data-icon="stop"');
    expect(html).toContain("hover:bg-foreground/10");
  });

  it("forwards extra button props like disabled", () => {
    const html = renderToStaticMarkup(
      createElement(
        HoverActionButton,
        { title: "Retry", onClick: () => {}, disabled: true },
        "Retry",
      ),
    );
    expect(html).toContain("disabled");
  });
});
