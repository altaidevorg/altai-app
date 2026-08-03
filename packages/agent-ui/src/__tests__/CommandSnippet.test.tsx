import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { CommandSnippet } from "../components/CommandSnippet.js";

describe("CommandSnippet", () => {
  it("falls back to /name when meta is missing", () => {
    const html = renderToStaticMarkup(
      createElement(CommandSnippet, { name: "unknown-cmd" }),
    );
    expect(html).toContain("/unknown-cmd");
    expect(html).toContain("font-mono");
  });

  it("renders invocation, label, and icon when meta is provided", () => {
    const html = renderToStaticMarkup(
      createElement(CommandSnippet, {
        name: "init",
        meta: {
          invocation: "/init",
          label: "Initialize workspace",
          icon: createElement("span", { "data-icon": "sparkles" }, "*"),
        },
      }),
    );
    expect(html).toContain("/init");
    expect(html).toContain("Initialize workspace");
    expect(html).toContain('data-icon="sparkles"');
  });
});
