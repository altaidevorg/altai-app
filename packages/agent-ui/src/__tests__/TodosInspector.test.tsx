import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { TodosInspector } from "../components/TodosInspector.js";

describe("TodosInspector", () => {
  it("renders empty state when total is 0", () => {
    const html = renderToStaticMarkup(
      createElement(TodosInspector, { done: 0, total: 0, todos: [] }),
    );
    expect(html).toContain("task checklist will appear here");
  });

  it("renders progress and todo rows", () => {
    const html = renderToStaticMarkup(
      createElement(TodosInspector, {
        done: 1,
        total: 2,
        todos: [
          { id: "1", title: "Done item", status: "completed" },
          { id: "2", title: "Active item", status: "in_progress" },
        ],
      }),
    );
    expect(html).toContain("1/2");
    expect(html).toContain("Done item");
    expect(html).toContain("Active item");
    expect(html).toContain("line-through");
    expect(html).toContain("bg-success");
    expect(html).toContain("bg-info");
  });

  it("sets progress bar width from done/total", () => {
    const html = renderToStaticMarkup(
      createElement(TodosInspector, {
        done: 1,
        total: 4,
        todos: [{ id: "1", title: "Quarter", status: "completed" }],
      }),
    );
    expect(html).toContain("width:25%");
  });
});
