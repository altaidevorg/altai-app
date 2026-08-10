/**
 * Pure Automations create-form status and submit labels (A6.258).
 */

/** Ready/status line when schedule is valid (no host error). */
export function automationCreateStatusText(ownerChatId: string | null): string {
  return ownerChatId
    ? "Schedule is ready"
    : "Select a chat to create one";
}

export function automationCreateSubmitLabel(creating: boolean): string {
  return creating ? "Creating…" : "Create";
}

export const AUTOMATION_CREATE_SESSION_FALLBACK_TITLE = "New chat";
