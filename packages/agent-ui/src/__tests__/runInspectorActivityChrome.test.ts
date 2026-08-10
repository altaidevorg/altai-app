import { describe, expect, it } from "vitest";
import {
  countCompletedTodos,
  filterActivityByKind,
  filterActivityByQuery,
  isAgentRunBusy,
} from "../lib/runInspectorActivityChrome.js";

describe("countCompletedTodos", () => {
  it("counts completed only", () => {
    expect(
      countCompletedTodos([
        { status: "completed" },
        { status: "pending" },
        { status: "completed" },
      ]),
    ).toBe(2);
  });
});

describe("filterActivityByQuery", () => {
  const items = [
    { label: "Search web", kind: "research", detail: "query: foo" },
    { label: "Call tool", kind: "mcp", tone: "ok" },
  ];
  it("matches fields case-insensitively", () => {
    expect(filterActivityByQuery(items, "WEB").map((i) => i.kind)).toEqual([
      "research",
    ]);
    expect(filterActivityByQuery(items, "").length).toBe(2);
  });
});

describe("filterActivityByKind", () => {
  it("filters kind", () => {
    expect(
      filterActivityByKind(
        [{ kind: "mcp" }, { kind: "research" }],
        "mcp",
      ).map((i) => i.kind),
    ).toEqual(["mcp"]);
  });
});

describe("isAgentRunBusy", () => {
  it("detects thinking/streaming", () => {
    expect(isAgentRunBusy("thinking")).toBe(true);
    expect(isAgentRunBusy("streaming")).toBe(true);
    expect(isAgentRunBusy("idle")).toBe(false);
  });
});
