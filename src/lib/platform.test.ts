import { describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/plugin-os", () => ({
  platform: () => "windows",
}));

import {
  IS_WINDOWS,
  USE_CUSTOM_WINDOW_CONTROLS,
  usesCustomWindowControls,
} from "./platform";

describe("Windows window chrome", () => {
  it("uses app-owned controls on Windows (no classic OS title bar)", () => {
    expect(IS_WINDOWS).toBe(true);
    expect(USE_CUSTOM_WINDOW_CONTROLS).toBe(true);
    expect(usesCustomWindowControls("windows")).toBe(true);
  });

  it("uses custom controls on Linux and Windows, not macOS", () => {
    expect(usesCustomWindowControls("linux")).toBe(true);
    expect(usesCustomWindowControls("windows")).toBe(true);
    expect(usesCustomWindowControls("macos")).toBe(false);
    expect(usesCustomWindowControls("")).toBe(false);
  });
});
