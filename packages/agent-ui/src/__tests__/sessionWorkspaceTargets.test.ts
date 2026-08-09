import { describe, expect, it } from "vitest";
import {
  sessionListWorkspaceTargets,
  sessionWorkspacePathForId,
} from "../lib/sessionWorkspaceTargets.js";

describe("sessionListWorkspaceTargets", () => {
  it("includes undefined plus unique paths", () => {
    expect(
      sessionListWorkspaceTargets([
        { workspacePath: "/a" },
        { workspacePath: null },
        { workspacePath: "/a" },
        { workspacePath: "/b" },
      ]),
    ).toEqual([undefined, "/a", "/b"]);
  });
});

describe("sessionWorkspacePathForId", () => {
  it("returns path or undefined", () => {
    expect(
      sessionWorkspacePathForId(
        [
          { id: "1", workspacePath: "/x" },
          { id: "2", workspacePath: null },
        ],
        "1",
      ),
    ).toBe("/x");
    expect(sessionWorkspacePathForId([{ id: "1" }], "9")).toBeUndefined();
  });
});
