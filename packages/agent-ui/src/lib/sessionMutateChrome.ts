/**
 * Pure helpers: prefer soft-archive over hard-delete for chat sessions (A6.80).
 */

export type SessionRemoveMode = "archive" | "delete" | "unavailable";

export function resolveSessionRemoveMode(input: {
  canArchive: boolean;
  canDelete: boolean;
}): SessionRemoveMode {
  if (input.canArchive) {
    return "archive";
  }
  if (input.canDelete) {
    return "delete";
  }
  return "unavailable";
}

export function sessionRemoveErrorMessage(mode: SessionRemoveMode): string {
  if (mode === "unavailable") {
    return "Remove session is unavailable on this host.";
  }
  if (mode === "archive") {
    return "Archive session failed.";
  }
  return "Delete session failed.";
}
