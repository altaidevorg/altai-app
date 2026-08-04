import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import {
  ApprovalsInspector,
  approvalPreview,
} from "../components/ApprovalsInspector.js";

describe("approvalPreview", () => {
  it("pretty-prints JSON payloads", () => {
    expect(approvalPreview({ path: "a.ts" })).toContain('"path"');
  });

  it("truncates long payloads", () => {
    const long = "x".repeat(1000);
    const result = approvalPreview({ data: long });
    expect(result.length).toBeLessThanOrEqual(902);
    expect(result.endsWith("…")).toBe(true);
  });

  it("falls back when JSON.stringify fails", () => {
    const circular: { self?: unknown } = {};
    circular.self = circular;
    expect(approvalPreview(circular)).toContain("[object Object]");
  });
});

describe("ApprovalsInspector", () => {
  it("renders empty state when no approvals", () => {
    const html = renderToStaticMarkup(
      createElement(ApprovalsInspector, {
        approvals: [],
        onRespond: () => {},
      }),
    );
    expect(html).toContain(
      "Actions that need your approval will appear here without interrupting",
    );
  });

  it("renders action, payload preview, and buttons", () => {
    const html = renderToStaticMarkup(
      createElement(ApprovalsInspector, {
        approvals: [
          { id: "a1", action: "write_file", payload: { path: "x.ts" } },
        ],
        onRespond: () => {},
      }),
    );
    expect(html).toContain("write_file");
    expect(html).toContain("x.ts");
    expect(html).toContain("Deny");
    expect(html).toContain("Approve");
    expect(html).toContain("animate-pulse");
  });
});
