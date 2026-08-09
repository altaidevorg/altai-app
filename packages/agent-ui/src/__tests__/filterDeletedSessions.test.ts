import { describe, expect, it } from "vitest";
import {
  appendDeletedSessionId,
  filterDeletedSessions,
} from "../lib/filterDeletedSessions.js";

describe("filterDeletedSessions", () => {
  it("removes blocked ids", () => {
    expect(
      filterDeletedSessions([{ id: "a" }, { id: "b" }], ["b"]),
    ).toEqual([{ id: "a" }]);
  });
});

describe("appendDeletedSessionId", () => {
  it("dedupes append", () => {
    expect(appendDeletedSessionId(["a"], "a")).toEqual(["a"]);
    expect(appendDeletedSessionId(["a"], "b")).toEqual(["a", "b"]);
  });
});
