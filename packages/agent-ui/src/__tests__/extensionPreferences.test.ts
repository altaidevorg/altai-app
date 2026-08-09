import { describe, expect, it } from "vitest";
import {
  coerceExtensionPreferences,
  defaultExtensionPreferences,
  isExtensionSettingKey,
  isValidSettingValue,
  parseSnippetsJson,
} from "../lib/extensionPreferences.js";

describe("extensionPreferences", () => {
  it("defaults and coerces", () => {
    const d = defaultExtensionPreferences();
    expect(d.autoFocusComposer).toBe(true);
    const c = coerceExtensionPreferences({ highContrast: true });
    expect(c.highContrast).toBe(true);
    expect(c.autoFocusComposer).toBe(true);
  });
  it("parses snippets JSON", () => {
    expect(
      parseSnippetsJson('[{"handle":"hi","body":"hello"}]'),
    ).toEqual([{ id: "snippet-0", handle: "hi", body: "hello" }]);
  });
  it("validates setting keys/values", () => {
    expect(isExtensionSettingKey("openPanelOnStartup")).toBe(true);
    expect(isValidSettingValue("openPanelOnStartup", true)).toBe(true);
    expect(isValidSettingValue("agentHostPath", 1)).toBe(false);
  });
});
