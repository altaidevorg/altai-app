import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import {
  parseTodoItems,
  summarizeTodos,
  TodoChecklist,
} from "../components/TodoChecklist.js";

describe("TodoChecklist", () => {
  it("normalizes free-form todo_write input", () => {
    const items = parseTodoItems({
      items: [
        { content: "A", status: "done" },
        { title: "B", status: "in-progress" },
        { task: "C", status: "pending" },
      ],
    });
    expect(items.map((i) => [i.title, i.status])).toEqual([
      ["A", "completed"],
      ["B", "in_progress"],
      ["C", "pending"],
    ]);
    expect(summarizeTodos(items)).toEqual({
      total: 3,
      done: 1,
      inProgress: 1,
      pct: 33,
    });
  });

  it("renders titles", () => {
    const html = renderToStaticMarkup(
      createElement(TodoChecklist, {
        items: [
          { id: "1", title: "Ship checklist", status: "completed" },
          { id: "2", title: "Wire hosts", status: "in_progress" },
        ],
      }),
    );
    expect(html).toContain("Ship checklist");
    expect(html).toContain("Wire hosts");
  });
});
