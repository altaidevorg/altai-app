import { describe, expect, it } from "vitest";
import {
  isTimedCacheFresh,
  taskTitleFromPrompt,
} from "../lib/timedCacheChrome.js";

describe("isTimedCacheFresh", () => {
  it("compares elapsed to ttl", () => {
    expect(isTimedCacheFresh(1000, 500, 1400)).toBe(true);
    expect(isTimedCacheFresh(1000, 500, 1500)).toBe(false);
  });
});

describe("taskTitleFromPrompt", () => {
  it("uses first trimmed line", () => {
    expect(taskTitleFromPrompt("  Hello\nWorld  ")).toBe("Hello");
    expect(taskTitleFromPrompt("   ")).toBe("");
  });
});
