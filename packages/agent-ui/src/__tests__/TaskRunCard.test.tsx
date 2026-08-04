import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { formatTaskAge, TaskRunCard } from "../components/TaskRunCard.js";

describe("formatTaskAge", () => {
  it("formats relative ages", () => {
    const now = 1_700_000_000_000;
    expect(formatTaskAge(now - 30_000, now)).toBe("now");
    expect(formatTaskAge(now - 5 * 60_000, now)).toBe("5m");
    expect(formatTaskAge(now - 3 * 3_600_000, now)).toBe("3h");
    expect(formatTaskAge(now - 2 * 86_400_000, now)).toBe("2d");
  });
});

describe("TaskRunCard", () => {
  it("renders active run affordances", () => {
    const html = renderToStaticMarkup(
      createElement(TaskRunCard, {
        title: "Fix auth bug",
        status: "running",
        createdAtMs: 1_700_000_000_000 - 120_000,
        tokens: 1500,
        subagentCount: 2,
        step: "Reading files",
        active: true,
        onOpen: () => {},
        onReuse: () => {},
        onStop: () => {},
        nowMs: 1_700_000_000_000,
      }),
    );
    expect(html).toContain("Fix auth bug");
    expect(html).toContain("Working");
    expect(html).toContain("1.5k tokens");
    expect(html).toContain("2 agents");
    expect(html).toContain("Reading files");
    expect(html).toContain("Stop");
    expect(html).toContain("2m");
  });

  it("renders finished failed run with retry/remove", () => {
    const html = renderToStaticMarkup(
      createElement(TaskRunCard, {
        title: "Add tests",
        status: "failed",
        createdAtMs: 1,
        outcome: { changesCount: 1, checksPassed: 0, checksFailed: 2 },
        onOpen: () => {},
        onReuse: () => {},
        onRetry: () => {},
        onRemove: () => {},
        nowMs: 1,
      }),
    );
    expect(html).toContain("Failed");
    expect(html).toContain("Retry");
    expect(html).toContain("Remove Add tests");
    expect(html).toContain("2 checks failed");
  });
});
