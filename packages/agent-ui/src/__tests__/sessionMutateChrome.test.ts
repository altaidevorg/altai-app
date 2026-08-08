import { describe, expect, it } from "vitest";
import {
  resolveSessionRemoveMode,
  sessionRemoveErrorMessage,
} from "../lib/sessionMutateChrome.js";

describe("resolveSessionRemoveMode", () => {
  it("prefers archive when available", () => {
    expect(
      resolveSessionRemoveMode({ canArchive: true, canDelete: true }),
    ).toBe("archive");
  });

  it("falls back to delete", () => {
    expect(
      resolveSessionRemoveMode({ canArchive: false, canDelete: true }),
    ).toBe("delete");
  });

  it("returns unavailable with neither capability", () => {
    expect(
      resolveSessionRemoveMode({ canArchive: false, canDelete: false }),
    ).toBe("unavailable");
  });
});

describe("sessionRemoveErrorMessage", () => {
  it("is host-facing copy", () => {
    expect(sessionRemoveErrorMessage("unavailable")).toMatch(/unavailable/i);
    expect(sessionRemoveErrorMessage("archive")).toMatch(/Archive/i);
  });
});
