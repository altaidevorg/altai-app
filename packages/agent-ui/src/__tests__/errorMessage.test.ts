import { describe, expect, it } from "vitest";
import {
  errorMessageFromUnknown,
  mutationKey,
  normalizedWorkspacePath,
} from "../lib/errorMessage.js";

describe("errorMessage helpers", () => {
  it("formats errors and keys", () => {
    expect(errorMessageFromUnknown(new Error("x"))).toBe("x");
    expect(errorMessageFromUnknown("y")).toBe("y");
    expect(mutationKey("job", "1")).toBe("job:1");
    expect(normalizedWorkspacePath(" /a ")).toBe("/a");
    expect(normalizedWorkspacePath("  ")).toBeNull();
  });
});
