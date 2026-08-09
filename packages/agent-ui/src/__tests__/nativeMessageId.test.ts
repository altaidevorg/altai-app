import { describe, expect, it } from "vitest";
import { newNativeMessageId } from "../lib/nativeMessageId.js";

describe("newNativeMessageId", () => {
  it("stable format with injectables", () => {
    expect(newNativeMessageId(() => 1000, () => 0.123456789)).toMatch(
      /^native-1000-[a-z0-9]+$/,
    );
    expect(newNativeMessageId(() => 1000, () => 0.123456789)).toBe(
      newNativeMessageId(() => 1000, () => 0.123456789),
    );
  });
});
