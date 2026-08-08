/**
 * Pure validation for Operations schedule create form drafts (A6.102).
 */

export type AutomationScheduleDraft =
  | { kind: "once"; at: string }
  | { kind: "every"; everyMs: number };

export type AutomationDraft = {
  title: string;
  prompt: string;
  schedule: AutomationScheduleDraft;
  enabled: boolean;
};

export type AutomationDraftResult =
  | { ok: true; draft: AutomationDraft }
  | { ok: false; error: string };

const MAX_TITLE = 120;
const MAX_PROMPT = 20_000;
const MIN_EVERY_MS = 60_000;
const MAX_EVERY_MS = 30 * 24 * 60 * 60 * 1000;

/** Interval presets shown in the compose form (label → everyMs). */
export const AUTOMATION_INTERVAL_PRESETS = [
  { label: "Hourly", everyMs: 60 * 60 * 1000 },
  { label: "Daily", everyMs: 24 * 60 * 60 * 1000 },
  { label: "Weekly", everyMs: 7 * 24 * 60 * 60 * 1000 },
] as const;

/**
 * Trim and validate before `work.createAutomation`.
 * Schedule "once" expects an ISO-8601 datetime string.
 */
export function validateAutomationDraft(input: {
  title: string;
  prompt: string;
  scheduleKind: "once" | "every";
  onceAt: string;
  everyMs: number;
  enabled?: boolean;
}): AutomationDraftResult {
  const title = input.title.trim();
  const prompt = input.prompt.trim();
  if (!title) {
    return { ok: false, error: "Title is required." };
  }
  if (title.length > MAX_TITLE) {
    return { ok: false, error: `Title must be at most ${MAX_TITLE} characters.` };
  }
  if (!prompt) {
    return { ok: false, error: "Instruction prompt is required." };
  }
  if (prompt.length > MAX_PROMPT) {
    return {
      ok: false,
      error: `Prompt must be at most ${MAX_PROMPT} characters.`,
    };
  }

  let schedule: AutomationScheduleDraft;
  if (input.scheduleKind === "once") {
    const raw = input.onceAt.trim();
    if (!raw) {
      return { ok: false, error: "Schedule time is required." };
    }
    const ms = Date.parse(raw);
    if (!Number.isFinite(ms)) {
      return { ok: false, error: "Schedule time is invalid." };
    }
    schedule = { kind: "once", at: new Date(ms).toISOString() };
  } else {
    const everyMs = input.everyMs;
    if (
      !Number.isFinite(everyMs) ||
      everyMs < MIN_EVERY_MS ||
      everyMs > MAX_EVERY_MS
    ) {
      return {
        ok: false,
        error: "Choose a repeat interval between 1 minute and 30 days.",
      };
    }
    schedule = { kind: "every", everyMs };
  }

  return {
    ok: true,
    draft: {
      title,
      prompt,
      schedule,
      enabled: input.enabled !== false,
    },
  };
}

/**
 * Owner conversation id for a new automation (host protocol requires chat_id).
 */
export function newAutomationOwnerChatId(): string {
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
    return `altai:auto-chat-${crypto.randomUUID()}`;
  }
  return `altai:auto-chat-${Date.now().toString(36)}`;
}
