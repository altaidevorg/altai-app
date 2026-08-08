import { describe, expect, it } from "vitest";
import {
  compactHostStatusLabel,
  nextSurfaceAfterSettingsToggle,
  settingsGearPressed,
  shouldShowSurfaceTextTabs,
} from "../lib/shellChrome.js";

describe("shellChrome", () => {
  it("hides VS Code surface text tabs (Desktop density)", () => {
    expect(shouldShowSurfaceTextTabs()).toBe(false);
  });

  it("toggles settings surface with gear press semantics", () => {
    expect(settingsGearPressed("settings")).toBe(true);
    expect(settingsGearPressed("chat")).toBe(false);
    expect(nextSurfaceAfterSettingsToggle("chat")).toBe("settings");
    expect(nextSurfaceAfterSettingsToggle("settings")).toBe("chat");
    expect(nextSurfaceAfterSettingsToggle("operations")).toBe("settings");
  });

  it("omits host chip when ready", () => {
    expect(compactHostStatusLabel("ready")).toBeNull();
    expect(compactHostStatusLabel("starting")).toBe("Starting…");
    expect(compactHostStatusLabel("error", "boom")).toBe("Host error");
  });
});
