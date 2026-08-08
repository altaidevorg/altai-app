/**
 * Pure composer attach-menu capability policy (A6.64).
 * Hosts supply capability flags; package owns mount rules (no dead menus).
 */

export type ComposerAttachCapabilityFlags = {
  canActiveFile: boolean;
  canSelection: boolean;
  canGitDiff: boolean;
  canTerminal: boolean;
};

/** Whether any attach surface should show the context control. */
export function canMountComposerAttachMenu(
  flags: ComposerAttachCapabilityFlags,
): boolean {
  return (
    flags.canActiveFile ||
    flags.canSelection ||
    flags.canGitDiff ||
    flags.canTerminal
  );
}

export type ComposerAttachSurface = "attachments" | "toolbar" | "all";

export function composerAttachSurfaceShowsAttachments(
  surface: ComposerAttachSurface,
): boolean {
  return surface === "attachments" || surface === "all";
}

export function composerAttachSurfaceShowsToolbar(
  surface: ComposerAttachSurface,
): boolean {
  return surface === "toolbar" || surface === "all";
}
