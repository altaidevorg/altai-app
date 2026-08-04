import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { InspectorEmpty } from "../components/InspectorEmpty.js";

describe("InspectorEmpty", () => {
  it("renders children with compact empty styling", () => {
    const html = renderToStaticMarkup(
      createElement(InspectorEmpty, null, "No changes yet"),
    );
    expect(html).toContain("No changes yet");
    expect(html).toContain("px-2 py-8 text-center");
    expect(html).toContain("text-[11px]");
    expect(html).toContain("text-muted-foreground");
  });

  it("renders without children", () => {
    const html = renderToStaticMarkup(createElement(InspectorEmpty));
    expect(html).toContain("px-2 py-8");
  });
});
