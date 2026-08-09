/**
 * Pure key helpers for dismissible Chat / side-panel chrome (A6.75, A6.137).
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


/**
 * Whether Escape should close the side panel (vs text field edit or open menus).
 * Hosts compute DOM facts; this helper stays pure.
 */
export function shouldDismissSidePanelOnEscape(input: {
  key: string;
  metaKey?: boolean;
  ctrlKey?: boolean;
  altKey?: boolean;
  /** INPUT / TEXTAREA / contenteditable targets should not close the panel. */
  isEditableTarget?: boolean;
  /** Open menu/listbox/dialog or data-state=open ancestor should win first. */
  hasOpenOverlay?: boolean;
}): boolean {
  if (!isEscapeDismissKey(input)) {
    return false;
  }
  if (input.isEditableTarget) {
    return false;
  }
  if (input.hasOpenOverlay) {
    return false;
  }
  return true;
}


/** DOM facts-only: whether focus is in a text-editing control. */
export function isTextEditingKeyboardTarget(input: {
  tagName?: string | null;
  isContentEditable?: boolean;
}): boolean {
  if (input.isContentEditable) {
    return true;
  }
  const tag = (input.tagName ?? "").toUpperCase();
  return tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT";
}
