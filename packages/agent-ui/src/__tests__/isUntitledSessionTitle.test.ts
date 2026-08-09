import { describe, expect, it } from "vitest";
import { DEFAULT_SESSION_TITLE } from "../lib/backendSessionTitle.js";
import { isUntitledSessionTitle } from "../lib/isUntitledSessionTitle.js";

describe("isUntitledSessionTitle", () => {
  it("treats empty and default as untitled", () => {
    expect(isUntitledSessionTitle(null)).toBe(true);
    expect(isUntitledSessionTitle("")).toBe(true);
    expect(isUntitledSessionTitle(DEFAULT_SESSION_TITLE)).toBe(true);
    expect(isUntitledSessionTitle("Work")).toBe(false);
  });
});
