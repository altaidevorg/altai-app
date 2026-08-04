import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { PlanRow } from "../components/PlanRow.js";

describe("PlanRow", () => {
  const baseProps = {
    path: "src/components/Button.tsx",
    kind: "edit",
    isNewFile: false,
    originalContent: "old line\nremoved line",
    proposedContent: "old line\nadded line",
    stats: { added: 1, removed: 1 },
    busy: false,
    onOpenDiff: () => {},
    onApply: () => {},
    onReject: () => {},
  };

  it("renders basename and full path", () => {
    const html = renderToStaticMarkup(
      createElement(PlanRow, baseProps),
    );
    expect(html).toContain("Button.tsx");
    expect(html).toContain("src/components/Button.tsx");
  });

  it("renders diff stats when provided", () => {
    const html = renderToStaticMarkup(
      createElement(PlanRow, baseProps),
    );
    expect(html).toContain("+1");
    expect(html).toContain("−1");
    expect(html).toContain("edit");
  });

  it("renders new badge for new files", () => {
    const html = renderToStaticMarkup(
      createElement(PlanRow, { ...baseProps, isNewFile: true }),
    );
    expect(html).toContain("new");
  });

  it("renders description for directory creation", () => {
    const html = renderToStaticMarkup(
      createElement(PlanRow, {
        ...baseProps,
        kind: "create_directory",
        isNewFile: false,
        stats: null,
        description: "Create new folder",
      }),
    );
    expect(html).toContain("Create new folder");
  });

  it("renders action buttons", () => {
    const html = renderToStaticMarkup(
      createElement(PlanRow, baseProps),
    );
    expect(html).toContain('aria-label="Open full diff"');
    expect(html).toContain('aria-label="Reject"');
    expect(html).toContain('aria-label="Apply this change"');
  });
});
