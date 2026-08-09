import { describe, expect, it } from "vitest";
import {
  backendSessionTitle,
  DEFAULT_SESSION_TITLE,
  displaySessionTitle,
} from "../lib/backendSessionTitle.js";

describe("backendSessionTitle", () => {
  it("trims or defaults", () => {
    expect(backendSessionTitle(" Hi ")).toBe("Hi");
    expect(backendSessionTitle("")).toBe(DEFAULT_SESSION_TITLE);
    expect(backendSessionTitle(null)).toBe(DEFAULT_SESSION_TITLE);
    expect(displaySessionTitle(" ")).toBe(DEFAULT_SESSION_TITLE);
    expect(displaySessionTitle("Tasks")).toBe("Tasks");
  });
});
