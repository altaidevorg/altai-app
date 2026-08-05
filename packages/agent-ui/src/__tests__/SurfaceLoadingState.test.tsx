import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { SurfaceLoadingState } from "../components/SurfaceLoadingState.js";

describe("SurfaceLoadingState", () => {
  it("renders panel density label", () => {
    const html = renderToStaticMarkup(
      createElement(SurfaceLoadingState, {
        density: "panel",
        children: "Loading tasks…",
      }),
    );
    expect(html).toContain("Loading tasks…");
    expect(html).toContain("py-8");
  });

  it("renders inline density", () => {
    const html = renderToStaticMarkup(
      createElement(SurfaceLoadingState, {
        density: "inline",
        children: "Loading automations…",
      }),
    );
    expect(html).toContain("Loading automations…");
    expect(html).toContain("text-[10px]");
    expect(html).not.toContain("py-8");
  });
});
