import { describe, expect, it } from "vitest";
import { isPlanModeOffTail } from "../lib/planModeTail.js";

describe("isPlanModeOffTail", () => {
  it("detects off/exit", () => {
    expect(isPlanModeOffTail("off")).toBe(true);
    expect(isPlanModeOffTail(" EXIT ")).toBe(true);
    expect(isPlanModeOffTail("on")).toBe(false);
    expect(isPlanModeOffTail("")).toBe(false);
  });
});
