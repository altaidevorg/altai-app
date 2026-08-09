/**
 * Pure plan-mode slash toast copy (A6.184).
 */

export function planModeOnToast(): string {
  return "Plan mode on";
}

export function planModeOffToast(): string {
  return "Plan mode off";
}

/** Toast after `/plan` toggle given resulting active flag. */
export function planModeToggleToast(active: boolean): string {
  return active ? planModeOnToast() : planModeOffToast();
}
