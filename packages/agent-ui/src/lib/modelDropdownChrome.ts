/**
 * Pure Model dropdown trigger / empty-state copy (A6.247).
 */

/** Trigger button label for Auto vs fixed model. */
export function modelDropdownTriggerLabel(
  autoSelected: boolean,
  currentLabel: string,
  autoModelLabel?: string | null,
): string {
  if (autoSelected) {
    return `Auto · ${autoModelLabel ?? currentLabel}`;
  }
  return currentLabel;
}

/** Trigger tooltip for Auto / usable / missing-key states. */
export function modelDropdownTriggerTitle(
  autoSelected: boolean,
  triggerUsable: boolean,
  currentLabel: string,
  autoModelLabel?: string | null,
): string {
  if (autoSelected) {
    return `Auto selects a compatible model for each task. Current recommendation: ${autoModelLabel ?? currentLabel}`;
  }
  if (triggerUsable) {
    return `Model: ${currentLabel}`;
  }
  return `${currentLabel} — add an API key in Model settings`;
}

/** Empty-list message, or null when results are present. */
export function modelDropdownEmptyMessage(
  filteredCount: number,
  availableCount: number,
  constraintMessage?: string | null,
): string | null {
  if (filteredCount !== 0) return null;
  if (availableCount === 0) {
    return "No models available — add an API key in Model settings.";
  }
  return constraintMessage ?? "No models match.";
}

/** Detail line under the Auto option row. */
export function modelDropdownAutoDetail(
  autoModelLabel?: string | null,
): string {
  return autoModelLabel
    ? `Recommended now: ${autoModelLabel}`
    : "Choose from compatible models";
}
