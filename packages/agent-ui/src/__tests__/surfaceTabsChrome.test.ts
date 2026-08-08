import { describe, expect, it } from "vitest";
import { nextAltaiSurface } from "../lib/surfaceTabsChrome.js";

describe("nextAltaiSurface", () => {
  it("moves with arrows and jumps with Home/End", () => {
    expect(nextAltaiSurface("chat", "ArrowRight")).toBe("operations");
    expect(nextAltaiSurface("settings", "ArrowLeft")).toBe("operations");
    expect(nextAltaiSurface("operations", "Home")).toBe("chat");
    expect(nextAltaiSurface("chat", "End")).toBe("settings");
  });
});
