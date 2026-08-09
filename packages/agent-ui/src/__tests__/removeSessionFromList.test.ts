import { describe, expect, it } from "vitest";
import {
  nextActiveIdAfterDelete,
  removeSessionFromList,
} from "../lib/removeSessionFromList.js";

describe("removeSessionFromList", () => {
  it("filters id", () => {
    expect(removeSessionFromList([{ id: "a" }, { id: "b" }], "a")).toEqual([
      { id: "b" },
    ]);
  });
});

describe("nextActiveIdAfterDelete", () => {
  it("picks next when active deleted", () => {
    expect(
      nextActiveIdAfterDelete([{ id: "a" }, { id: "b" }], "a", "a"),
    ).toBe("b");
  });
  it("keeps active when other deleted", () => {
    expect(
      nextActiveIdAfterDelete([{ id: "a" }, { id: "b" }], "b", "a"),
    ).toBe("a");
  });
  it("null when empty", () => {
    expect(nextActiveIdAfterDelete([{ id: "a" }], "a", "a")).toBeNull();
  });
});
