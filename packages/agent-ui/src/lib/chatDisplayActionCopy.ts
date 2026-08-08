/**
 * Pure display-message action copy (A6.53).
 * Hosts bind buttons; package owns stable labels/titles.
 */

export function displayCopyActionLabel(copied: boolean): string {
  return copied ? "Copied" : "Copy";
}

export function displayOpenFileActionTitle(filePath?: string | null): string {
  return filePath ? `Open ${filePath}` : "Open file";
}

export function displayOpenDiffActionTitle(filePath?: string | null): string {
  return filePath ? `Review diff for ${filePath}` : "Open diff";
}

export function displayOpeningActionLabel(opening: boolean, idle: string): string {
  return opening ? "Opening…" : idle;
}

export function displayDiffReviewTitle(filePath?: string | null): string {
  return filePath ? `ALTAI · ${filePath}` : "ALTAI review";
}
