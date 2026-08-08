/**
 * Pure helpers for capability-gated edit-checkpoint list/restore chrome (A6.92).
 */

import type { CheckpointInfo } from "@altai/host-contract";
import type { CheckpointItem } from "../components/CheckpointMenuPanel.js";

export type CheckpointChromeFlags = {
  canList: boolean;
  canRestore: boolean;
  hasActiveChat: boolean;
};

/**
 * Show the checkpoints affordance only when list is available and a chat is
 * open. Restore may be absent independently (control still opens, restore
 * disabled inside when the host omits restore).
 */
export function canMountCheckpointChrome(
  flags: CheckpointChromeFlags,
): boolean {
  return flags.canList && flags.hasActiveChat;
}

export function canRestoreCheckpoint(
  flags: CheckpointChromeFlags,
  restoringId: string | null,
): boolean {
  return flags.canRestore && flags.hasActiveChat && restoringId === null;
}

/** Map host-contract checkpoints into shared CheckpointMenuPanel items. */
export function toCheckpointMenuItems(
  rows: readonly CheckpointInfo[],
): CheckpointItem[] {
  return rows.map((row) => {
    const createdMs = Date.parse(row.createdAt);
    return {
      id: row.id,
      path: row.label && row.label.trim() ? row.label : row.id,
      label: row.label && row.label.trim() ? row.label : "Checkpoint",
      createdMs: Number.isFinite(createdMs) ? createdMs : 0,
    };
  });
}

/**
 * When the native list includes a filesystem path, prefer it for basename
 * display (Desktop parity). Host-contract only carries optional label, so the
 * VS Code adapter stores path (or path · tool) in label.
 */
export function preferredCheckpointLabel(input: {
  path?: string;
  label?: string;
}): string | undefined {
  const path = input.path?.trim();
  const label = input.label?.trim();
  if (path && label && path !== label) {
    return `${path} · ${label}`;
  }
  return path || label || undefined;
}
