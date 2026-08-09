import { describe, expect, it } from "vitest";
import {
  CLEARED_OUTPUT,
  estimateTokens,
  isClearedOutput,
} from "../lib/tokenEstimate.js";

describe("tokenEstimate (A6.145)", () => {
  it("estimates tokens and detects cleared markers", () => {
    expect(estimateTokens("")).toBe(0);
    expect(estimateTokens("abcd")).toBe(1);
    expect(estimateTokens("abcde")).toBe(2);
    expect(isClearedOutput(CLEARED_OUTPUT)).toBe(true);
    expect(isClearedOutput({ cleared: true })).toBe(true);
    expect(isClearedOutput(null)).toBe(false);
  });
});
