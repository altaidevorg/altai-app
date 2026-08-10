import { describe, expect, it } from "vitest";
import {
  compactionFailedDetail,
  compactionFailedLabel,
  compactionRequestedDetail,
  compactionRequestedLabel,
} from "../lib/compactionToast.js";

describe("compactionToast", () => {
  it("stable copy", () => {
    expect(compactionRequestedLabel()).toMatch(/compact/i);
    expect(compactionRequestedDetail()).toMatch(/runtime/i);
    expect(compactionFailedLabel()).toMatch(/failed/i);
    expect(compactionFailedDetail(new Error("x"))).toBe("x");
    expect(compactionFailedDetail("y")).toBe("y");
  });
});
