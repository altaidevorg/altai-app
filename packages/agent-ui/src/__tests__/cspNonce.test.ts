import { describe, expect, it } from "vitest";
import { createNonce } from "../lib/cspNonce.js";

describe("createNonce", () => {
  it("returns length", () => {
    expect(createNonce(16)).toHaveLength(16);
  });
});
