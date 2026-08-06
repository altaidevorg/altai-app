/**
 * Composer Enter-key routing (send / steer / queue).
 * Wave 4 / A6.10 — host supplies availability; UI only.
 */

export type ComposerAction = "send" | "steer" | "queue";

export type ComposerActionAvailability = {
  isRunning: boolean;
  isBusy: boolean;
  canSend: boolean;
  canSteer: boolean;
  canQueue: boolean;
};

export function resolveComposerEnterAction(input: {
  availability: ComposerActionAvailability;
  shiftKey: boolean;
  modifierKey: boolean;
}): ComposerAction | null {
  if (input.shiftKey) return null;
  if (input.modifierKey && input.availability.isRunning) {
    return input.availability.canSteer ? "steer" : null;
  }
  if (input.availability.isBusy) {
    return input.availability.canQueue ? "queue" : null;
  }
  return input.availability.canSend ? "send" : null;
}

/**
 * After a send is accepted, keep only typed draft that was not part of the
 * submitted payload (user kept typing during the request).
 */
export function remainingTextAfterAcceptedDispatch(
  current: string,
  submitted: string,
  draftWasUnchanged: boolean,
): string {
  if (draftWasUnchanged) return "";
  if (submitted && current.startsWith(submitted)) {
    return current.slice(submitted.length).replace(/^\s+/, "");
  }
  return current;
}
