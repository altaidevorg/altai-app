import { describe, expect, it } from "vitest";
import {
  requestMethodFromInit,
  requestUrlToString,
} from "../lib/requestInitChrome.js";

describe("requestUrlToString", () => {
  it("handles string, URL, and Request-like", () => {
    expect(requestUrlToString("https://x")).toBe("https://x");
    expect(requestUrlToString(new URL("https://y/z"))).toBe("https://y/z");
    expect(requestUrlToString({ url: "https://r" })).toBe("https://r");
  });
});

describe("requestMethodFromInit", () => {
  it("defaults and uppercases", () => {
    expect(requestMethodFromInit(undefined)).toBe("GET");
    expect(requestMethodFromInit({ method: "post" })).toBe("POST");
  });
});
