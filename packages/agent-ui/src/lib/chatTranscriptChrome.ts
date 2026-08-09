/**
 * Pure AiChat transcript chrome policy (aria-live, retry, error variant).
 * Wave 4 / A6.32 — hosts supply run outcome + prefs; no React.
 */

/** Announce preference for transcript aria-live (accepts unknown host values). */
export type TranscriptAriaLivePref = "off" | "polite" | "assertive" | string;

/**
 * Map host announce prefs to a valid `aria-live` value for the transcript.
 */
export function resolveChatAriaLive(
  announce: TranscriptAriaLivePref,
): "off" | "polite" | "assertive" {
  if (announce === "off") return "off";
  if (announce === "assertive") return "assertive";
  return "polite";
}

/**
 * Whether the last assistant bubble may show Retry (host knows outcome).
 */
export function canRetryLastAssistantTurn(input: {
  retryableFailure: boolean;
  role: string;
  index: number;
  messageCount: number;
  status: string;
}): boolean {
  return (
    input.retryableFailure &&
    input.role === "assistant" &&
    input.index === input.messageCount - 1 &&
    input.status !== "streaming"
  );
}

/**
 * Join AI SDK-style text parts (Desktop UIMessage user turns).
 */
export function joinMessageTextParts(
  parts: readonly { type?: string; text?: string }[],
  separator = "\n",
): string {
  return parts
    .filter((p) => p.type === "text" && typeof p.text === "string")
    .map((p) => p.text as string)
    .join(separator);
}

/**
 * Recoverable terminal copy that must not render as a hard error.
 * Shared with AgentStatusPill defaults.
 */
export function isRecoverableAttentionMessage(message: string): boolean {
  return message.startsWith("Run paused");
}

export type TranscriptRunErrorVariant = "error" | "attention";

export function resolveTranscriptRunErrorVariant(
  message: string,
): TranscriptRunErrorVariant {
  return isRecoverableAttentionMessage(message) ? "attention" : "error";
}

/**
 * Generic retryable failed-run check used by Desktop agent runs store.
 * Outcome shapes match host bridge contracts.
 */
export function isRetryableRunOutcome(
  outcome:
    | { kind: string; retryable?: boolean }
    | null
    | undefined,
): boolean {
  return outcome?.kind === "failed" && outcome.retryable === true;
}
