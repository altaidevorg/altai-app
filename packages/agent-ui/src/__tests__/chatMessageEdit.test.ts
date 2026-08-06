import { describe, expect, it } from "vitest";
import {
  canEditUserMessage,
  parseUserTurnId,
  renumberUserTurnIds,
  truncateBoundaryForEdit,
  truncateDisplayAfterUserTurn,
} from "../lib/chatMessageEdit.js";

describe("parseUserTurnId", () => {
  it("accepts user:N including 0", () => {
    expect(parseUserTurnId("user:0")).toBe(0);
    expect(parseUserTurnId("user:2")).toBe(2);
    expect(parseUserTurnId("message:2")).toBeNull();
  });
});

describe("truncateBoundaryForEdit", () => {
  it("maps edit turn to keep-before boundary id", () => {
    expect(truncateBoundaryForEdit(1)).toBe("user:0");
    expect(truncateBoundaryForEdit(3)).toBe("user:2");
    expect(truncateBoundaryForEdit(0)).toBeNull();
  });
});

describe("renumberUserTurnIds", () => {
  it("numbers user turns in order", () => {
    const input = [
      { id: "a", role: "user", content: "one" },
      { id: "b", role: "assistant", content: "r1" },
      { id: "c", role: "user", content: "two" },
    ];
    expect(renumberUserTurnIds(input).map((m) => m.id)).toEqual([
      "user:1",
      "b",
      "user:2",
    ]);
  });
});

describe("truncateDisplayAfterUserTurn", () => {
  it("drops the edited turn and everything after", () => {
    const msgs = [
      { id: "user:1", role: "user", content: "a" },
      { id: "x", role: "assistant", content: "A" },
      { id: "user:2", role: "user", content: "b" },
      { id: "y", role: "assistant", content: "B" },
    ];
    expect(truncateDisplayAfterUserTurn(msgs, 2)).toEqual([
      { id: "user:1", role: "user", content: "a" },
      { id: "x", role: "assistant", content: "A" },
    ]);
    expect(truncateDisplayAfterUserTurn(msgs, 1)).toEqual([]);
  });
});

describe("canEditUserMessage", () => {
  it("requires truncate + start and no active run", () => {
    expect(
      canEditUserMessage({
        role: "user",
        canTruncate: true,
        canStartRun: true,
        runActive: false,
      }),
    ).toBe(true);
    expect(
      canEditUserMessage({
        role: "assistant",
        canTruncate: true,
        canStartRun: true,
        runActive: false,
      }),
    ).toBe(false);
    expect(
      canEditUserMessage({
        role: "user",
        canTruncate: true,
        canStartRun: true,
        runActive: true,
      }),
    ).toBe(false);
  });
});
