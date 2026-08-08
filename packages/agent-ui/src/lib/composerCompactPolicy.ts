/**
 * Pure composer context-compaction policy (A6.59).
 * Hosts supply capability + chat id presence + busy; package owns mount rules.
 */

export type ComposerCompactFlags = {
  canCompact: boolean;
  hasActiveChat: boolean;
  busy: boolean;
};

/**
 * Mount CompactNowControl only when the host advertises compaction and a chat
 * is active. Architecture forbids always-visible disabled placeholders when the
 * capability is missing.
 */
export function canMountCompactControl(flags: ComposerCompactFlags): boolean {
  return flags.canCompact && flags.hasActiveChat;
}

/** Control may be clicked when mounted and not mid-request. */
export function canInvokeCompact(flags: ComposerCompactFlags): boolean {
  return canMountCompactControl(flags) && !flags.busy;
}
