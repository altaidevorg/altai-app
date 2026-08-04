import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { TaskOutcome } from "../components/TaskOutcome.js";

describe("TaskOutcome", () => {
  it("renders file changes and passed checks", () => {
    const html = renderToStaticMarkup(
      createElement(TaskOutcome, {
        changesCount: 3,
        checksPassed: 2,
        checksFailed: 0,
      }),
    );
    expect(html).toContain("3 files changed");
    expect(html).toContain("2 check passed");
    expect(html).toContain("text-success");
  });

  it("renders singular file change", () => {
    const html = renderToStaticMarkup(
      createElement(TaskOutcome, {
        changesCount: 1,
        checksPassed: 0,
        checksFailed: 0,
      }),
    );
    expect(html).toContain("1 file changed");
    expect(html).toContain("No checks reported");
  });

  it("renders failed checks with destructive styling", () => {
    const html = renderToStaticMarkup(
      createElement(TaskOutcome, {
        changesCount: 0,
        checksPassed: 1,
        checksFailed: 2,
      }),
    );
    expect(html).toContain("No file changes");
    expect(html).toContain("2 check failed");
    expect(html).toContain("text-destructive");
  });

  it("renders no changes and no checks", () => {
    const html = renderToStaticMarkup(
      createElement(TaskOutcome, {
        changesCount: 0,
        checksPassed: 0,
        checksFailed: 0,
      }),
    );
    expect(html).toContain("No file changes");
    expect(html).toContain("No checks reported");
  });
});
