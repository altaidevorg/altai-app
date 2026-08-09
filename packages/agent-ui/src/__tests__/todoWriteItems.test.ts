import { describe, expect, it } from "vitest";
import { parseTodoWriteItems } from "../lib/todoWriteItems.js";

describe("parseTodoWriteItems", () => {
  it("prefers content/title/task/text for title", () => {
    const out = parseTodoWriteItems(
      [
        { content: "A" },
        { title: "B", status: "done" },
        { task: "C", id: "c1" },
        { text: "D", description: "desc" },
        {},
      ],
      "sess",
    );
    expect(out[0]).toMatchObject({ id: "sess:0", title: "A", status: "pending" });
    expect(out[1]).toMatchObject({ title: "B", status: "completed" });
    expect(out[2]).toMatchObject({ id: "c1", title: "C" });
    expect(out[3]).toMatchObject({ title: "D", description: "desc" });
    expect(out[4]).toMatchObject({ title: "Untitled task" });
  });
});
