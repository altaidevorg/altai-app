import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { InspectorMetric } from "../components/InspectorMetric.js";

describe("InspectorMetric", () => {
  it("renders label and value with metric styling", () => {
    const html = renderToStaticMarkup(
      createElement(InspectorMetric, { label: "Plan", value: "3/5" }),
    );
    expect(html).toContain("Plan");
    expect(html).toContain("3/5");
    expect(html).toContain("uppercase");
    expect(html).toContain("tabular-nums");
  });

  it("renders em-dash placeholders", () => {
    const html = renderToStaticMarkup(
      createElement(InspectorMetric, { label: "Subagents", value: "—" }),
    );
    expect(html).toContain("Subagents");
    expect(html).toContain("—");
  });
});
