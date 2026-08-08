import { describe, expect, it } from "vitest";
import { createSecureId, getSecureRandomBytes } from "../lib/secureRandom.js";

describe("secureRandom", () => {
  it("returns requested length", () => {
    expect(getSecureRandomBytes(8)).toHaveLength(8);
  });
  it("prefixes secure ids", () => {
    expect(createSecureId("msg")).toMatch(/^msg-/);
  });
});
