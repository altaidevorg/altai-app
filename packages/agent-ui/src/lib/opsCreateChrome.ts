/**
 * Pure Operations create-form readiness + run model auto pick (A6.218).
 */

export function canCreateAutomationDraft(input: {
  ownerChatId: string | null | undefined;
  message: string;
  creating: boolean;
  scheduleError: string | null | undefined;
}): boolean {
  return Boolean(
    input.ownerChatId &&
      input.message.trim() &&
      !input.creating &&
      !input.scheduleError,
  );
}

export function canCreateTaskDraft(input: {
  prompt: string;
  creating: boolean;
}): boolean {
  return Boolean(input.prompt.trim() && !input.creating);
}

export type RunModelPickable = { id: string };

/**
 * When auto is off, return the requested id. When auto is on, pick from
 * resolvable models then fall back to the requested id.
 */
export function resolveRunModelIdFromCandidates<T extends RunModelPickable>(input: {
  requestedModelId: string;
  useAuto: boolean;
  listResolvable: () => readonly T[];
  pick: (models: readonly T[]) => T | null;
}): string {
  if (!input.useAuto) return input.requestedModelId;
  const resolvable = input.listResolvable();
  return input.pick(resolvable)?.id ?? input.requestedModelId;
}
