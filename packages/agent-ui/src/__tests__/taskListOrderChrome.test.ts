import { describe, expect, it } from "vitest";
import {
  filterEnabledAgents,
  filterTaskSourceAssignments,
  sortByCreatedAtDesc,
} from "../lib/taskListOrderChrome.js";

describe("sortByCreatedAtDesc", () => {
  it("orders newest first", () => {
    expect(
      sortByCreatedAtDesc([
        { id: "a", createdAt: 1 },
        { id: "b", createdAt: 3 },
        { id: "c", createdAt: 2 },
      ]).map((x) => x.id),
    ).toEqual(["b", "c", "a"]);
  });
});

describe("filterEnabledAgents", () => {
  it("drops disabled ids", () => {
    expect(
      filterEnabledAgents(
        [{ id: "a" }, { id: "b" }],
        (id) => id === "a",
      ).map((a) => a.id),
    ).toEqual(["b"]);
  });
});

describe("filterTaskSourceAssignments", () => {
  it("keeps task sources only", () => {
    expect(
      filterTaskSourceAssignments([
        { id: "1", source: { kind: "task" } },
        { id: "2", source: { kind: "github" } },
      ]).map((a) => a.id),
    ).toEqual(["1"]);
  });
});
