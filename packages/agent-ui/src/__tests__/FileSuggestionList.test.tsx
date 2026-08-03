import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { FileSuggestionList } from "../components/FileSuggestionList.js";

const iconUrlForFile = (name: string) => `icon://${name}`;

describe("FileSuggestionList", () => {
  it("shows no-workspace empty state", () => {
    const html = renderToStaticMarkup(
      createElement(FileSuggestionList, {
        files: [],
        activeIndex: 0,
        indexing: false,
        truncated: false,
        hasWorkspace: false,
        onPick: () => {},
        onHover: () => {},
        iconUrlForFile,
      }),
    );
    expect(html).toContain("No workspace open");
  });

  it("renders file rows, active highlight, and truncation footer", () => {
    const html = renderToStaticMarkup(
      createElement(FileSuggestionList, {
        files: ["src/app.ts", "README.md"],
        activeIndex: 1,
        indexing: false,
        truncated: true,
        hasWorkspace: true,
        onPick: () => {},
        onHover: () => {},
        iconUrlForFile,
      }),
    );
    expect(html).toContain("Workspace files");
    expect(html).toContain("app.ts");
    expect(html).toContain("src");
    expect(html).toContain("README.md");
    expect(html).toContain('src="icon://README.md"');
    expect(html).toContain("bg-foreground/[0.065]");
    expect(html).toContain("Workspace is large");
  });

  it("shows indexing state when the list is empty", () => {
    const html = renderToStaticMarkup(
      createElement(FileSuggestionList, {
        files: [],
        activeIndex: 0,
        indexing: true,
        truncated: false,
        hasWorkspace: true,
        onPick: () => {},
        onHover: () => {},
        iconUrlForFile,
        indexingIndicator: createElement("span", { "data-spin": "1" }, "..."),
      }),
    );
    expect(html).toContain("Indexing workspace");
    expect(html).toContain('data-spin="1"');
  });
});
