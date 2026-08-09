import { describe, expect, it } from "vitest";
import { mergeRecoveredSessions } from "../lib/mergeRecoveredSessions.js";

describe("mergeRecoveredSessions", () => {
  it("recovers backend-only sessions newest first", () => {
    const frontend = [
      { id: "f1", title: "Front", createdAt: 10, updatedAt: 10 },
    ];
    const backend = [
      { id: "f1", updatedAt: 99, title: "ignored" },
      { id: "b2", updatedAt: 50, title: "  Back  " },
      { id: "b1", updatedAt: 20, title: "" },
    ];
    const { merged, recoveredIds } = mergeRecoveredSessions(
      frontend,
      backend,
      [],
    );
    expect(recoveredIds).toEqual(["b2", "b1"]);
    expect(merged.map((s) => s.id)).toEqual(["b2", "b1", "f1"]);
    expect(merged[0]).toMatchObject({ title: "Back", createdAt: 50 });
    expect(merged[1]).toMatchObject({ title: "New chat" });
    // frontend not mutated
    expect(frontend).toHaveLength(1);
  });

  it("honors delete blocklist", () => {
    const { recoveredIds, merged } = mergeRecoveredSessions(
      [],
      [{ id: "x", updatedAt: 1, title: "X" }],
      ["x"],
    );
    expect(recoveredIds).toEqual([]);
    expect(merged).toEqual([]);
  });
});
