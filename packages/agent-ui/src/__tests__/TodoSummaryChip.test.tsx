import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { TodoSummaryChip } from "../components/TodoSummaryChip.js";

describe("TodoSummaryChip", () => {
  it("renders nothing for an empty plan", () => {
    const html = renderToStaticMarkup(
      createElement(TodoSummaryChip, { todos: [] }),
    );
    expect(html).toBe("");
  });

  it("shows progress and checklist titles", () => {
    const html = renderToStaticMarkup(
      createElement(TodoSummaryChip, {
        todos: [
          { id: "1", title: "Ship checklist", status: "completed" },
          { id: "2", title: "Wire hosts", status: "in_progress" },
        ],
      }),
    );
    expect(html).toContain("1/2");
    expect(html).toContain("Ship checklist");
    expect(html).toContain("Wire hosts");
    expect(html).toContain("Plan: 1 of 2 tasks done");
  });
});
