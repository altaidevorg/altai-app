/**
 * Composer Enter-key routing (send / steer / queue).
 * Wave 4 / A6.10 — host supplies availability; UI only.
 */

export type ComposerAction = "send" | "steer" | "queue";

export type ComposerActionAvailability = {
  isRunning: boolean;
  isBusy: boolean;
  /** True while cancel has been requested but the run still holds the session. */
  isCancelling: boolean;
  canSend: boolean;
  canSteer: boolean;
  canQueue: boolean;
};

export type ComposerActionAvailabilityInput = {
  status: string;
  hasDraft: boolean;
  hasNativeAttachment: boolean;
  runId: string | null;
  submitting: boolean;
};

/**
 * Derive send / steer / queue availability from host run state.
 * Shared by Desktop `useComposer` and ports-first host adapters.
 */
export function getComposerActionAvailability(
  input: ComposerActionAvailabilityInput,
): ComposerActionAvailability {
  const isRunning = input.status === "thinking" || input.status === "streaming";
  const isAwaiting = input.status === "awaiting-approval";
  const isCancelling = input.status === "cancelling";
  // Treat approval waits as busy so typed input queues/steers instead of
  // looking like a fresh idle send.
  const isBusy = isRunning || isCancelling || isAwaiting;
  const ready = input.hasDraft && !input.submitting;
  return {
    isBusy,
    isRunning,
    isCancelling,
    canSend: ready && !isBusy,
    canSteer:
      ready &&
      isRunning &&
      input.runId !== null &&
      !input.hasNativeAttachment,
    canQueue: ready && isBusy,
  };
}

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
