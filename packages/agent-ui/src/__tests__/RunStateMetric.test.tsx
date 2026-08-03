import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { RunStateMetric } from "../components/RunStateMetric.js";

describe("RunStateMetric", () => {
  it("renders label and value with run state styling", () => {
    const html = renderToStaticMarkup(
      createElement(RunStateMetric, { label: "Input", value: "12,345" }),
    );
    expect(html).toContain("Input");
    expect(html).toContain("12,345");
    expect(html).toContain("tabular-nums");
    expect(html).toContain("bg-foreground/[0.035]");
  });

  it("renders zero values without dropping the tile", () => {
    const html = renderToStaticMarkup(
      createElement(RunStateMetric, { label: "Approvals", value: "0" }),
    );
    expect(html).toContain("Approvals");
    expect(html).toContain(">0<");
  });
});
