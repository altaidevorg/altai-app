/**
 * Pure helpers for capability-gating the permission-mode switcher (A6.78).
 */

export type PermissionSwitcherFlags = {
  permissionModes: boolean;
  settingsGet: boolean;
  settingsUpdate: boolean;
};

/**
 * Show the shared PermissionModeSwitcher only when the host can both load the
 * current mode and persist changes. Read-only switchers look enabled but do
 * nothing — architecture forbids that.
 */
export function canMountPermissionModeSwitcher(
  flags: PermissionSwitcherFlags,
): boolean {
  return flags.permissionModes && flags.settingsGet && flags.settingsUpdate;
}
