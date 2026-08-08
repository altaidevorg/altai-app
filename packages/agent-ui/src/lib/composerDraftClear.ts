/**
 * Pure draft clearance after a composer submit plan is accepted or handled
 * locally (A6.34). Keeps residual typing when the user edits during flight.
 */

import { remainingTextAfterAcceptedDispatch } from "./composerEnterAction.js";
import { removeAcceptedItems } from "./composerDraft.js";
import type { ComposerSubmitSnapshot } from "./composerSubmitPlan.js";

export type ComposerDraftState = ComposerSubmitSnapshot & {
  /** Monotonic host revision for the draft text field. */
  valueRevision: number;
};

/**
 * Apply post-accept draft clearance for text, file chips, snippets, and
 * command chips. Pure: no stores or HostPorts.
 *
 * Chip removal is reference-based (same objects as the accepted snapshot),
 * matching the Desktop submit snapshot pattern.
 */
export function clearComposerDraftAfterAccept(
  current: ComposerDraftState,
  accepted: ComposerDraftState,
): ComposerSubmitSnapshot {
  return {
    value: remainingTextAfterAcceptedDispatch(
      current.value,
      accepted.value,
      current.valueRevision === accepted.valueRevision,
    ),
    files: removeAcceptedItems(current.files, accepted.files),
    snippets: removeAcceptedItems(current.snippets, accepted.snippets),
    commands: removeAcceptedItems(current.commands, accepted.commands),
  };
}
