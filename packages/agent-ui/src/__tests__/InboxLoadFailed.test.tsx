import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { InboxLoadFailed } from "../components/InboxLoadFailed.js";

describe("InboxLoadFailed", () => {
  it("renders error icon, message, and retry button", () => {
    const html = renderToStaticMarkup(
      createElement(InboxLoadFailed, { onRetry: () => {} }),
    );
    expect(html).toContain("Inbox could not be loaded");
    expect(html).toContain("Try again");
    expect(html).toContain("<svg");
    expect(html).toContain("bg-destructive/10");
  });

  it("wires onClick on retry button", () => {
    let clicked = false;
    const html = renderToStaticMarkup(
      createElement(InboxLoadFailed, { onRetry: () => { clicked = true; } }),
    );
    expect(html).toContain('type="button"');
    expect(clicked).toBe(false);
  });
});
