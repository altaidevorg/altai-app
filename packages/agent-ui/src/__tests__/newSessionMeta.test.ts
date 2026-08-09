import { describe, expect, it } from "vitest";
import { DEFAULT_SESSION_TITLE } from "../lib/backendSessionTitle.js";
import { newUntitledSessionMeta } from "../lib/newSessionMeta.js";

describe("newUntitledSessionMeta", () => {
  it("fills New chat + timestamps", () => {
    const m = newUntitledSessionMeta({ id: "s1", now: 42, workspacePath: "/w" });
    expect(m).toEqual({
      id: "s1",
      title: DEFAULT_SESSION_TITLE,
      createdAt: 42,
      updatedAt: 42,
      workspacePath: "/w",
    });
  });
});
