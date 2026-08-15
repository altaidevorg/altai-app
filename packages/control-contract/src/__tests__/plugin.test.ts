import { describe, expect, test } from "vitest";
import {
  allowedCapabilities,
  diffPluginUpgrade,
  expandsCapabilities,
  isVersionAdvance,
  validatePluginManifest,
  type PluginManifest,
} from "../plugin.js";

const manifest = (
  kind: PluginManifest["kind"],
  capabilities: PluginManifest["capabilities"],
): PluginManifest => ({
  plugin_id: { type: "plugin_id", value: "plg_demo" },
  kind,
  version: { major: 1, minor: 0, patch: 0 },
  display_name: "Demo plugin",
  capabilities,
});

describe("plugin manifest", () => {
  test("agent content and application plugins are distinct", () => {
    expect(
      validatePluginManifest(manifest("application", ["jobs", "webhooks", "scoped_secrets", "plugin_ui"])),
    ).toBeNull();
    expect(validatePluginManifest(manifest("agent_content", []))).toBeNull();
    expect(allowedCapabilities("agent_content")).toEqual([]);
    for (const capability of allowedCapabilities("application")) {
      expect(validatePluginManifest(manifest("agent_content", [capability]))).toEqual({
        type: "capability_not_allowed_for_kind",
        kind: "agent_content",
        capability,
      });
    }
  });

  test("duplicate capabilities are rejected", () => {
    expect(validatePluginManifest(manifest("application", ["jobs", "jobs"]))).toEqual({
      type: "duplicate_capability",
      capability: "jobs",
    });
  });

  test("empty display names are rejected", () => {
    expect(
      validatePluginManifest({ ...manifest("application", []), display_name: "   " }),
    ).toEqual({ type: "empty_display_name" });
  });
});

describe("plugin upgrade disclosure", () => {
  test("discloses capability expansion", () => {
    const previous = manifest("application", ["jobs"]);
    const next: PluginManifest = {
      ...previous,
      version: { major: 1, minor: 1, patch: 0 },
      capabilities: ["jobs", "webhooks"],
    };
    const disclosure = diffPluginUpgrade(previous, next);
    expect(disclosure.added_capabilities).toEqual(["webhooks"]);
    expect(disclosure.removed_capabilities).toEqual([]);
    expect(isVersionAdvance(disclosure)).toBe(true);
    expect(expandsCapabilities(disclosure)).toBe(true);
  });

  test("a pure version bump neither adds nor removes", () => {
    const previous = manifest("application", ["jobs"]);
    const next = { ...previous, version: { major: 1, minor: 0, patch: 1 } };
    const disclosure = diffPluginUpgrade(previous, next);
    expect(expandsCapabilities(disclosure)).toBe(false);
    expect(isVersionAdvance(disclosure)).toBe(true);
  });

  test("versions order numerically, not lexicographically", () => {
    const previous = manifest("application", ["jobs"]);
    const next = { ...previous, version: { major: 1, minor: 10, patch: 0 } };
    expect(isVersionAdvance(diffPluginUpgrade(previous, next))).toBe(true);
    const downgrade = { ...previous, version: { major: 0, minor: 99, patch: 0 } };
    expect(isVersionAdvance(diffPluginUpgrade(previous, downgrade))).toBe(false);
  });
});
