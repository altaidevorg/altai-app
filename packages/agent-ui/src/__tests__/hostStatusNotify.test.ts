import { describe, expect, it } from "vitest";
import { shouldNotifyHostRecovered } from "../lib/hostStatusNotify.js";

describe("shouldNotifyHostRecovered", () => {
  it("fires only error → ready", () => {
    expect(shouldNotifyHostRecovered("error", "ready")).toBe(true);
    expect(shouldNotifyHostRecovered(undefined, "ready")).toBe(false);
    expect(shouldNotifyHostRecovered("ready", "ready")).toBe(false);
    expect(shouldNotifyHostRecovered("error", "error")).toBe(false);
  });
});
