import { describe, expect, it } from "vitest";
import { listSettingsHubNav } from "../lib/settingsHubChrome.js";
import { filterSettingsNav } from "../lib/settingsSearchChrome.js";

describe("filterSettingsNav", () => {
  const nav = listSettingsHubNav({
    canProvider: true,
    canModel: true,
    canPermission: true,
    canMcp: true,
    canSkills: true,
  });

  it("returns all when query empty", () => {
    expect(filterSettingsNav(nav, "  ")).toHaveLength(nav.length);
  });

  it("finds context by compact keyword", () => {
    const hits = filterSettingsNav(nav, "compact");
    expect(hits.map((h) => h.id)).toContain("context");
  });

  it("finds accessibility by a11y", () => {
    const hits = filterSettingsNav(nav, "a11y");
    expect(hits.map((h) => h.id)).toEqual(["accessibility"]);
  });

  it("matches multi-token queries", () => {
    const hits = filterSettingsNav(nav, "agent host");
    expect(hits.some((h) => h.id === "general" || h.id === "host")).toBe(true);
  });
});
