import { describe, expect, it } from "vitest";
import {
  openedContextSettingsToast,
  openedMcpSettingsToast,
  openedModelSettingsToast,
  openedOperationsInboxToast,
  openedOperationsScheduledToast,
  openedOperationsWorkToast,
  openedPermissionSettingsToast,
  openedSkillsToast,
} from "../lib/slashSettingsToast.js";

describe("slashSettingsToast", () => {
  it("returns stable copy", () => {
    expect(openedOperationsWorkToast()).toMatch(/Operations work/);
    expect(openedOperationsInboxToast()).toMatch(/inbox/);
    expect(openedOperationsScheduledToast()).toMatch(/scheduled/);
    expect(openedModelSettingsToast()).toMatch(/model/);
    expect(openedPermissionSettingsToast()).toMatch(/permission/);
    expect(openedMcpSettingsToast()).toMatch(/MCP/);
    expect(openedSkillsToast()).toMatch(/skills/);
    expect(openedContextSettingsToast()).toMatch(/context/);
  });
});
