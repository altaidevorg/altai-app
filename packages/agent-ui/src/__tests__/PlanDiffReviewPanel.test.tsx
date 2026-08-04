import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import {
  PlanDiffReviewPanel,
  planDiffStats,
} from "../components/PlanDiffReviewPanel.js";

describe("planDiffStats", () => {
  it("counts coarse added/removed lines", () => {
    expect(planDiffStats("a\nb\n", "a\nc\n")).toEqual({
      added: 1,
      removed: 1,
    });
  });
});

describe("PlanDiffReviewPanel", () => {
  it("renders empty state", () => {
    const html = renderToStaticMarkup(
      createElement(PlanDiffReviewPanel, {
        queue: [],
        historyCount: 0,
        onApplyOne: () => {},
        onRejectOne: () => {},
        onOpenDiff: () => {},
      }),
    );
    expect(html).toContain("Change review");
    expect(html).toContain("No changes to review");
    expect(html).toContain("safe restore option");
    expect(html).not.toContain("Discard all");
  });

  it("renders pending queue actions and feedback", () => {
    const html = renderToStaticMarkup(
      createElement(PlanDiffReviewPanel, {
        queue: [
          {
            id: "q1",
            path: "/ws/src/App.tsx",
            kind: "edit",
            isNewFile: false,
            originalContent: "old\n",
            proposedContent: "new\n",
          },
        ],
        historyCount: 2,
        feedback: "Change applied.",
        onDiscardAll: () => {},
        onApplyAll: () => {},
        onApplyOne: () => {},
        onRejectOne: () => {},
        onOpenDiff: () => {},
        history: createElement("div", null, "History slot"),
      }),
    );
    expect(html).toContain("1 pending change");
    expect(html).toContain("Awaiting your decision");
    expect(html).toContain("Discard all");
    expect(html).toContain("Apply 1");
    expect(html).toContain("Change applied.");
    expect(html).toContain("History slot");
    expect(html).toContain("App.tsx");
  });
});
