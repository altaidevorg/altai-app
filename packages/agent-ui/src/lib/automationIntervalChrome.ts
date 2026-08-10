/**
 * Pure automation every-interval conversion (A6.223).
 */

const MS_PER_MINUTE = 60_000;

export function everyMsFromMinutes(minutes: number): number {
  return minutes * MS_PER_MINUTE;
}

export function minutesFromEveryMs(everyMs: number): number {
  return everyMs / MS_PER_MINUTE;
}

/** Stringify minutes for the create-form interval input. */
export function everyMinutesInputFromMs(everyMs: number): string {
  return String(minutesFromEveryMs(everyMs));
}
