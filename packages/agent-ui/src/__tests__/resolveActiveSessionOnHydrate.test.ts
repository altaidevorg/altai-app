import { describe, expect, it } from "vitest";
import { DEFAULT_SESSION_TITLE } from "../lib/backendSessionTitle.js";
import {
  createUntitledSessionMeta,
  resolveActiveSessionOnHydrate,
} from "../lib/resolveActiveSessionOnHydrate.js";

describe("resolveActiveSessionOnHydrate", () => {
  it("prefers activeId", () => {
    const sessions = [
      { id: "a", title: "A" },
      { id: "b", title: "B" },
    ];
    const r = resolveActiveSessionOnHydrate(sessions, "b", () =>
      createUntitledSessionMeta("x", 1),
    );
    expect(r.active.id).toBe("b");
    expect(r.created).toBe(false);
  });

  it("reuses untitled head", () => {
    const sessions = [{ id: "a", title: DEFAULT_SESSION_TITLE }];
    const r = resolveActiveSessionOnHydrate(sessions, null, () =>
      createUntitledSessionMeta("x", 1),
    );
    expect(r.active.id).toBe("a");
    expect(r.created).toBe(false);
  });

  it("creates when none", () => {
    const r = resolveActiveSessionOnHydrate([], null, () =>
      createUntitledSessionMeta("x", 1),
    );
    expect(r.created).toBe(true);
    expect(r.nextSessions).toHaveLength(1);
  });
});
