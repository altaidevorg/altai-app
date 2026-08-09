import { describe, expect, it } from "vitest";
import { headersInitToRecord } from "../lib/headersInit.js";

describe("headersInitToRecord", () => {
  it("handles undefined", () => {
    expect(headersInitToRecord(undefined)).toBeUndefined();
  });

  it("handles object and array forms", () => {
    expect(headersInitToRecord({ A: 1, b: "x" })).toEqual({ A: "1", b: "x" });
    expect(
      headersInitToRecord([
        ["X-A", "1"],
        ["X-B", "2"],
      ]),
    ).toEqual({ "X-A": "1", "X-B": "2" });
  });

  it("handles Headers-like forEach", () => {
    const h = {
      forEach(cb: (v: string, k: string) => void) {
        cb("v", "k");
      },
    };
    expect(headersInitToRecord(h)).toEqual({ k: "v" });
  });
});
