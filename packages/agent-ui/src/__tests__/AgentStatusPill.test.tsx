import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { AgentStatusPill } from "../components/AgentStatusPill.js";

const idle = {
  status: "idle" as const,
  step: null,
  approvalsPending: 0,
  error: null,
  activeSubagentCount: 0,
};

describe("AgentStatusPill", () => {
  it("renders nothing when idle and not busy", () => {
    const html = renderToStaticMarkup(
      createElement(AgentStatusPill, {
        meta: idle,
        formatStepLabel: (s) => s,
      }),
    );
    expect(html).toBe("");
  });

  it("shows friendly step label while thinking", () => {
    const html = renderToStaticMarkup(
      createElement(AgentStatusPill, {
        meta: {
          ...idle,
          status: "thinking",
          step: "exec",
        },
        formatStepLabel: (s) => (s === "exec" ? "Run" : s),
      }),
    );
    expect(html).toContain("Run");
    expect(html).toContain("Agent status: Run");
  });

  it("surfaces approval state", () => {
    const html = renderToStaticMarkup(
      createElement(AgentStatusPill, {
        meta: {
          ...idle,
          status: "awaiting-approval",
          approvalsPending: 2,
        },
        formatStepLabel: (s) => s,
        announce: false,
      }),
    );
    expect(html).toContain("2 approvals needed");
  });

  it("hides recoverable attention when hideError is set", () => {
    const html = renderToStaticMarkup(
      createElement(AgentStatusPill, {
        meta: {
          ...idle,
          status: "error",
          error: "Run paused — turn limit",
        },
        formatStepLabel: (s) => s,
        hideError: true,
      }),
    );
    expect(html).toBe("");
  });
});
