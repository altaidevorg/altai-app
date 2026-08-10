import { describe, expect, it } from "vitest";
import {
  datetimeLocalInputToMs,
  isFiniteNumber,
  parseFiniteMinutesInput,
} from "../lib/automationFormParseChrome.js";

describe("datetimeLocalInputToMs", () => {
  it("parses local datetime", () => {
    const ms = datetimeLocalInputToMs("2030-01-15T12:30");
    expect(isFiniteNumber(ms)).toBe(true);
    expect(new Date(ms).getFullYear()).toBe(2030);
  });

  it("returns NaN for garbage", () => {
    expect(isFiniteNumber(datetimeLocalInputToMs("not-a-date"))).toBe(false);
  });
});

describe("parseFiniteMinutesInput", () => {
  it("parses numbers", () => {
    expect(parseFiniteMinutesInput("60")).toBe(60);
    expect(isFiniteNumber(parseFiniteMinutesInput("x"))).toBe(false);
  });
});
