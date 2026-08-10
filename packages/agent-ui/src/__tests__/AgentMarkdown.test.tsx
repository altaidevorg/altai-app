import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { AgentMarkdown } from "../components/AgentMarkdown.js";

describe("AgentMarkdown", () => {
  it("renders GFM headings, tables, and fenced code through one shared renderer", () => {
    const html = renderToStaticMarkup(
      createElement(AgentMarkdown, {
        content: "## Status\n\n| Item | State |\n| --- | --- |\n| UI | Ready |\n\n```ts\nconst ready = true;\n```",
      }),
    );

    expect(html).toContain("Status");
    expect(html).toContain("<table");
    expect(html).toContain("const ready = true;");
  });

  it("delegates links to the host instead of granting navigation authority", () => {
    const seen: string[] = [];
    const html = renderToStaticMarkup(
      createElement(AgentMarkdown, {
        content: "[Open docs](https://example.com)",
        renderLink: ({ href, children }) => {
          seen.push(href);
          return createElement("span", { "data-link": href }, children);
        },
      }),
    );

    expect(seen).toEqual(["https://example.com/"]);
    expect(html).toContain('data-link="https://example.com/"');
  });
});
