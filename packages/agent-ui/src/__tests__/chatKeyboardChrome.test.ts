import { describe, expect, it } from "vitest";
import { isEscapeDismissKey } from "../lib/chatKeyboardChrome.js";

describe("isEscapeDismissKey", () => {
  it("matches plain Escape", () => {
    expect(isEscapeDismissKey({ key: "Escape" })).toBe(true);
    expect(isEscapeDismissKey({ key: "Escape", ctrlKey: true })).toBe(false);
    expect(isEscapeDismissKey({ key: "Enter" })).toBe(false);
  });
});
