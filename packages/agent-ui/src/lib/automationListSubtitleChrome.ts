/**
 * Pure Automations list chrome labels (A6.240).
 */

/** Subtitle when the scheduled-automations list is showing. */
export function automationListSubtitle(input: {
  repeat: number;
  once: number;
}): string {
  return `${input.repeat} recurring · ${input.once} one-time`;
}

/** Subtitle when the create form is open. */
export function automationCreateSubtitle(): string {
  return "Define an instruction, owner chat, and schedule";
}
