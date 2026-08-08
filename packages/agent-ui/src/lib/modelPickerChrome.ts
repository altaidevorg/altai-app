/**
 * Pure helpers for capability-gating and filtering the model picker (A6.87).
 */

import type { ModelInfo } from "@altai/host-contract";

export type ModelPickerFlags = {
  list: boolean;
  select: boolean;
  settingsGet: boolean;
};

/** AUTO sentinel used when host default model is unset / "auto". */
export const AUTO_MODEL_ID = "auto";

/**
 * Show the model picker only when the host can list models, load current
 * selection, and persist a new selection (no dead controls).
 */
export function canMountModelPicker(flags: ModelPickerFlags): boolean {
  return flags.list && flags.select && flags.settingsGet;
}

/**
 * Resolve the selected model id from host settings.
 */
export function resolveSelectedModelId(
  defaultModelId: string | undefined,
): string {
  const trimmed = defaultModelId?.trim();
  return trimmed && trimmed.length > 0 ? trimmed : AUTO_MODEL_ID;
}

/**
 * Case-insensitive id/label filter for the compact picker list.
 */
export function filterModels(
  models: readonly ModelInfo[],
  query: string,
): ModelInfo[] {
  const q = query.trim().toLowerCase();
  if (!q) {
    return [...models];
  }
  return models.filter((model) => {
    const hay = `${model.id} ${model.label} ${model.providerId}`.toLowerCase();
    return hay.includes(q);
  });
}

/**
 * Display label for the composer trigger.
 */
export function modelTriggerLabel(
  selectedId: string,
  models: readonly ModelInfo[],
): string {
  if (selectedId === AUTO_MODEL_ID) {
    return "Auto";
  }
  const match = models.find((model) => model.id === selectedId);
  return match?.label ?? selectedId;
}

/**
 * Model id to send on startRun. Omits `auto` / empty so the host may route.
 */
export function modelIdForStartRun(
  selectedId: string | null | undefined,
): string | undefined {
  const trimmed = selectedId?.trim();
  if (!trimmed || trimmed === AUTO_MODEL_ID) {
    return undefined;
  }
  return trimmed;
}
