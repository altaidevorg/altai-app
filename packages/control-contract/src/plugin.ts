import type { TypedId } from "./ids.js";

// Plugin manifest and capability contracts (package 071). Mirrors
// src-tauri/crates/altai-control-protocol/src/plugin.rs — keep both sides
// in sync.

export type PluginCapability =
  | "jobs"
  | "webhooks"
  | "scoped_secrets"
  | "plugin_ui";

export type PluginKind = "agent_content" | "application";

export type PluginVersion = {
  readonly major: number; readonly minor: number; readonly patch: number;
};

export type PluginManifest = {
  readonly plugin_id: TypedId; readonly kind: PluginKind;
  readonly version: PluginVersion; readonly display_name: string;
  readonly capabilities: readonly PluginCapability[];
};

export type PluginManifestError =
  | { readonly type: "duplicate_capability"; readonly capability: PluginCapability }
  | { readonly type: "capability_not_allowed_for_kind"; readonly kind: PluginKind; readonly capability: PluginCapability }
  | { readonly type: "empty_display_name" };

const APPLICATION_CAPABILITIES: readonly PluginCapability[] = [
  "jobs", "webhooks", "scoped_secrets", "plugin_ui",
];

export const allowedCapabilities = (kind: PluginKind): readonly PluginCapability[] =>
  kind === "agent_content" ? [] : APPLICATION_CAPABILITIES;

const compareVersion = (a: PluginVersion, b: PluginVersion): number =>
  a.major - b.major || a.minor - b.minor || a.patch - b.patch;

export const validatePluginManifest = (
  manifest: PluginManifest,
): PluginManifestError | null => {
  if (manifest.display_name.trim() === "") return { type: "empty_display_name" };
  const seen = new Set<PluginCapability>();
  for (const capability of manifest.capabilities) {
    if (seen.has(capability)) return { type: "duplicate_capability", capability };
    if (!allowedCapabilities(manifest.kind).includes(capability)) {
      return { type: "capability_not_allowed_for_kind", kind: manifest.kind, capability };
    }
    seen.add(capability);
  }
  return null;
};

export type PluginUpgradeDisclosure = {
  readonly plugin_id: TypedId;
  readonly from_version: PluginVersion;
  readonly to_version: PluginVersion;
  readonly added_capabilities: readonly PluginCapability[];
  readonly removed_capabilities: readonly PluginCapability[];
};

export const diffPluginUpgrade = (
  previous: PluginManifest,
  next: PluginManifest,
): PluginUpgradeDisclosure => {
  const previousSet = new Set(previous.capabilities);
  const nextSet = new Set(next.capabilities);
  // Deduplicate and sort in the Rust enum's declaration order so the JSON
  // matches the Rust disclosure (BTreeSet difference there) exactly.
  const order = (a: PluginCapability, b: PluginCapability) =>
    APPLICATION_CAPABILITIES.indexOf(a) - APPLICATION_CAPABILITIES.indexOf(b);
  const added = [...new Set(next.capabilities)]
    .filter((capability) => !previousSet.has(capability))
    .sort(order);
  const removed = [...new Set(previous.capabilities)]
    .filter((capability) => !nextSet.has(capability))
    .sort(order);
  return {
    plugin_id: next.plugin_id,
    from_version: previous.version,
    to_version: next.version,
    added_capabilities: added,
    removed_capabilities: removed,
  };
};

export const isVersionAdvance = (disclosure: PluginUpgradeDisclosure): boolean =>
  compareVersion(disclosure.to_version, disclosure.from_version) > 0;

export const expandsCapabilities = (disclosure: PluginUpgradeDisclosure): boolean =>
  disclosure.added_capabilities.length > 0;
