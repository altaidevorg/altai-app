import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { RunOverviewCard } from "../components/RunOverviewCard.js";

describe("RunOverviewCard", () => {
  it("renders token label, step, and metrics", () => {
    const html = renderToStaticMarkup(
      createElement(RunOverviewCard, {
        statusPill: createElement("span", null, "Running"),
        tokenLabel: "1,240 tokens",
        step: "Editing files",
        metrics: [
          { label: "Plan", value: "2/4" },
          { label: "Changes", value: "3" },
        ],
      }),
    );
    expect(html).toContain("Running");
    expect(html).toContain("1,240 tokens");
    expect(html).toContain("Editing files");
    expect(html).toContain("Plan");
    expect(html).toContain("2/4");
    expect(html).toContain("Changes");
  });

  it("omits step when empty", () => {
    const html = renderToStaticMarkup(
      createElement(RunOverviewCard, {
        statusPill: createElement("span", null, "Idle"),
        tokenLabel: "No usage yet",
        metrics: [],
      }),
    );
    expect(html).toContain("No usage yet");
    expect(html).not.toContain("line-clamp-2");
  });
});
