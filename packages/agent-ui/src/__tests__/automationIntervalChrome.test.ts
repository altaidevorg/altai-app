import { describe, expect, it } from "vitest";
import {
  everyMinutesInputFromMs,
  everyMsFromMinutes,
  minutesFromEveryMs,
} from "../lib/automationIntervalChrome.js";

describe("automation interval conversion", () => {
  it("round-trips minutes and ms", () => {
    expect(everyMsFromMinutes(60)).toBe(3_600_000);
    expect(minutesFromEveryMs(3_600_000)).toBe(60);
    expect(everyMinutesInputFromMs(120_000)).toBe("2");
  });
});
