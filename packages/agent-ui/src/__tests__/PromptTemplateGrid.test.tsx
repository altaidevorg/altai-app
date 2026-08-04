import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { PromptTemplateGrid } from "../components/PromptTemplateGrid.js";

describe("PromptTemplateGrid", () => {
  it("renders nothing without templates", () => {
    expect(
      renderToStaticMarkup(
        createElement(PromptTemplateGrid, {
          templates: [],
          onSelect: () => {},
        }),
      ),
    ).toBe("");
  });

  it("renders labels and compact three-column layout", () => {
    const html = renderToStaticMarkup(
      createElement(PromptTemplateGrid, {
        templates: [
          { label: "Fix a bug", value: "fix it" },
          { label: "Add tests", value: "test it" },
        ],
        onSelect: () => {},
        columns: 3,
        density: "compact",
      }),
    );
    expect(html).toContain("Fix a bug");
    expect(html).toContain("Add tests");
    expect(html).toContain("grid-cols-3");
    expect(html).toContain("text-[9px]");
  });
});
