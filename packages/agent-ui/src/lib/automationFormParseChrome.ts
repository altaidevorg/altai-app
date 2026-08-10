/**
 * Pure automation create-form field parsing (A6.237).
 */

/** `datetime-local` input string → epoch ms (may be NaN if invalid). */
export function datetimeLocalInputToMs(value: string): number {
  return new Date(value).getTime();
}

/** Repeat-minutes text field → number (may be NaN). */
export function parseFiniteMinutesInput(value: string): number {
  return Number(value);
}

/** Guard for schedule timestamps / derived intervals. */
export function isFiniteNumber(value: number): boolean {
  return Number.isFinite(value);
}
