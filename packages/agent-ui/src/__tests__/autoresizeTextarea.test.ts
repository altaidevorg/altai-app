import { describe, expect, it } from "vitest";
import { autoresizeTextarea } from "../lib/autoresizeTextarea.js";

function fakeTextarea(value: string, scrollHeight: number): HTMLTextAreaElement {
  const el = {
    value,
    scrollHeight,
    style: { height: "99px" },
  };
  return el as unknown as HTMLTextAreaElement;
}

describe("autoresizeTextarea", () => {
  it("no-ops for null", () => {
    expect(() => autoresizeTextarea(null)).not.toThrow();
  });

  it("clears height when empty", () => {
    const el = fakeTextarea("", 40);
    el.style.height = "80px";
    autoresizeTextarea(el);
    expect(el.style.height).toBe("");
  });

  it("caps height at maxPx", () => {
    const el = fakeTextarea("hello", 400);
    autoresizeTextarea(el, { maxPx: 176 });
    expect(el.style.height).toBe("176px");
  });

  it("uses scrollHeight when smaller than max", () => {
    const el = fakeTextarea("hi", 48);
    autoresizeTextarea(el, { maxPx: 176 });
    expect(el.style.height).toBe("48px");
  });
});
