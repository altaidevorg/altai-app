import { createElement } from "react";
import { describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import {
  ChatExternalLink,
  ChatPathLink,
} from "../components/ChatPathLink.js";

describe("ChatPathLink", () => {
  it("renders path and children", () => {
    const html = renderToStaticMarkup(
      createElement(ChatPathLink, {
        path: "src/app.ts",
        onOpen: () => {},
        children: "app.ts",
      }),
    );
    expect(html).toContain("app.ts");
    expect(html).toContain('title="src/app.ts"');
  });

  it("returns null for blank path", () => {
    const html = renderToStaticMarkup(
      createElement(ChatPathLink, { path: "  ", onOpen: () => {} }),
    );
    expect(html).toBe("");
  });
});

describe("ChatExternalLink", () => {
  it("renders href", () => {
    const onOpen = vi.fn();
    const html = renderToStaticMarkup(
      createElement(ChatExternalLink, {
        href: "https://example.com",
        onOpen,
      }),
    );
    expect(html).toContain("https://example.com");
    expect(html).toContain('href="https://example.com"');
  });
});
