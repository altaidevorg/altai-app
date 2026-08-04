import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { PlanModeStrip } from "../components/PlanModeStrip.js";

describe("PlanModeStrip", () => {
  it("renders nothing when inactive", () => {
    const html = renderToStaticMarkup(
      createElement(PlanModeStrip, {
        active: false,
        queueLen: 2,
        onReview: () => {},
        onExit: () => {},
      }),
    );
    expect(html).toBe("");
  });

  it("renders no-edits copy when queue is empty", () => {
    const html = renderToStaticMarkup(
      createElement(PlanModeStrip, {
        active: true,
        queueLen: 0,
        onReview: () => {},
        onExit: () => {},
      }),
    );
    expect(html).toContain("Plan mode");
    expect(html).toContain("no edits queued");
    expect(html).toContain("Exit");
    expect(html).not.toContain(">Review<");
  });

  it("renders queued count and Review button", () => {
    const html = renderToStaticMarkup(
      createElement(PlanModeStrip, {
        active: true,
        queueLen: 3,
        onReview: () => {},
        onExit: () => {},
      }),
    );
    expect(html).toContain("3 queued");
    expect(html).toContain("Review");
  });
});
