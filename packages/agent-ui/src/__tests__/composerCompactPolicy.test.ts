import { describe, expect, it } from "vitest";
import {
  canInvokeCompact,
  canMountCompactControl,
} from "../lib/composerCompactPolicy.js";

describe("canMountCompactControl", () => {
  it("requires capability and an active chat", () => {
    expect(
      canMountCompactControl({
        canCompact: true,
        hasActiveChat: true,
        busy: false,
      }),
    ).toBe(true);
    expect(
      canMountCompactControl({
        canCompact: false,
        hasActiveChat: true,
        busy: false,
      }),
    ).toBe(false);
    expect(
      canMountCompactControl({
        canCompact: true,
        hasActiveChat: false,
        busy: false,
      }),
    ).toBe(false);
  });
});

describe("canInvokeCompact", () => {
  it("disables while busy", () => {
    expect(
      canInvokeCompact({
        canCompact: true,
        hasActiveChat: true,
        busy: true,
      }),
    ).toBe(false);
    expect(
      canInvokeCompact({
        canCompact: true,
        hasActiveChat: true,
        busy: false,
      }),
    ).toBe(true);
  });
});
