import { describe, expect, it } from "vitest";
import {
  listSettingsHubNav,
  listSettingsHubSections,
  normalizeSettingsHubSection,
} from "../lib/settingsHubChrome.js";

describe("listSettingsHubNav", () => {
  it("lists Studio-parity sections", () => {
    const ids = listSettingsHubNav({
      canProvider: true,
      canModel: true,
      canPermission: true,
      canCompaction: true,
      canMcp: true,
      canSkills: true,
    }).map((item) => item.id);
    expect(ids).toEqual([
      "general",
      "shortcuts",
      "models",
      "context",
      "agents",
      "skills",
      "github",
      "languages",
      "mcp",
      "hooks",
      "accessibility",
      "host",
      "about",
    ]);
  });
});

describe("listSettingsHubSections", () => {
  it("returns every section id", () => {
    expect(
      listSettingsHubSections({
        canProvider: false,
        canModel: false,
        canPermission: false,
        canMcp: false,
        canSkills: false,
      }),
    ).toHaveLength(13);
  });
});

describe("normalizeSettingsHubSection", () => {
  it("maps legacy provider/model tabs to models", () => {
    const available = listSettingsHubSections({
      canProvider: true,
      canModel: true,
      canPermission: true,
      canMcp: true,
      canSkills: true,
    });
    expect(normalizeSettingsHubSection("provider", available)).toBe("models");
    expect(normalizeSettingsHubSection("permission", available)).toBe("models");
    expect(normalizeSettingsHubSection("about", available)).toBe("about");
  });
});
