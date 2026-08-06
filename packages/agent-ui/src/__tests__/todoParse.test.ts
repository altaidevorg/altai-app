import { describe, expect, it } from "vitest";
import {
  isTodoToolName,
  parseTodoItems,
  parseTodoItemsFromInput,
  summarizeTodoItems,
  summarizeTodos,
} from "../lib/todoParse.js";

describe("isTodoToolName", () => {
  it("recognizes todo tool names", () => {
    expect(isTodoToolName("todo_write")).toBe(true);
    expect(isTodoToolName("Update Todos")).toBe(true);
    expect(isTodoToolName("edit")).toBe(false);
  });
});

describe("parseTodoItems", () => {
  it("normalizes free-form todo_write input", () => {
    const items = parseTodoItems({
      items: [
        { content: "A", status: "done" },
        { title: "B", status: "in-progress" },
        { task: "C", status: "pending" },
      ],
    });
    expect(items).toEqual([
      { id: "item-0", title: "A", status: "completed" },
      { id: "item-1", title: "B", status: "in_progress" },
      { id: "item-2", title: "C", status: "pending" },
    ]);
    expect(summarizeTodos(items)).toEqual({
      total: 3,
      done: 1,
      inProgress: 1,
      pct: 33,
    });
  });

  it("aliases match primary names", () => {
    const input = { items: [{ content: "A", status: "done" }] };
    expect(parseTodoItemsFromInput(input)).toEqual(parseTodoItems(input));
    const items = parseTodoItems(input);
    expect(summarizeTodoItems(items)).toEqual(summarizeTodos(items));
  });
});
