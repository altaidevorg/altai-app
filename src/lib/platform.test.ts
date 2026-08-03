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
  it("keeps native recovery controls on Windows", () => {
    expect(IS_WINDOWS).toBe(true);
    expect(USE_CUSTOM_WINDOW_CONTROLS).toBe(false);
    expect(usesCustomWindowControls("windows")).toBe(false);
  });

  it("retains custom controls only for Linux", () => {
    expect(usesCustomWindowControls("linux")).toBe(true);
    expect(usesCustomWindowControls("macos")).toBe(false);
    expect(usesCustomWindowControls("")).toBe(false);
  });
});

