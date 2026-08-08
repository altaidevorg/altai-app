/**
 * Pure key helpers for dismissible Chat chrome (A6.75).
 */

export function isEscapeDismissKey(input: {
  key: string;
  metaKey?: boolean;
  ctrlKey?: boolean;
  altKey?: boolean;
}): boolean {
  if (input.metaKey || input.ctrlKey || input.altKey) {
    return false;
  }
  return input.key === "Escape";
}
