import { describe, expect, it } from "vitest";
import { newSessionId } from "../lib/sessionId.js";

describe("newSessionId", () => {
  it("formats deterministically with injected clock/random", () => {
    expect(newSessionId(() => 0x100, () => 0.5)).toMatch(/^s-[0-9a-z]+-[0-9a-z]+$/);
    expect(newSessionId(() => 1, () => 0)).toBe(
      `s-${(1).toString(36)}-${(0).toString(36).slice(2, 8)}`,
    );
  });
});
