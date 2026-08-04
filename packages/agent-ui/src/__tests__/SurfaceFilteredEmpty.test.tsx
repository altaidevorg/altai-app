import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { SurfaceFilteredEmpty } from "../components/SurfaceFilteredEmpty.js";

describe("SurfaceFilteredEmpty", () => {
  it("renders message and clear action", () => {
    const html = renderToStaticMarkup(
      createElement(SurfaceFilteredEmpty, {
        message: "No tasks match this view.",
        onClear: () => {},
      }),
    );
    expect(html).toContain("No tasks match this view.");
    expect(html).toContain("Clear filters");
  });
});
