/** Messages allowed across the webview/extension boundary. Keep this small and
 * validate every value because webview messages are untrusted input. */
export type ChatWebviewMessage =
  | { readonly type: "chat/send"; readonly prompt: string }
  | { readonly type: "chat/stop"; readonly runId: string }
  | { readonly type: "chat/removeContext"; readonly id: string }
  | { readonly type: "chat/openLogs" };

export type ChatContextChip = {
  readonly id: string;
  readonly kind: "selection" | "file" | "diagnostics";
  readonly label: string;
  readonly uri: string;
  readonly range?: { readonly startLine: number; readonly endLine: number };
};

export type ChatRunEvent =
  | { readonly type: "run_started"; readonly chatId: string; readonly runId: string; readonly seq: number }
  | { readonly type: "agent_message"; readonly chatId: string; readonly runId: string; readonly seq: number; readonly content: string }
  | { readonly type: "thinking"; readonly chatId: string; readonly runId: string; readonly seq: number; readonly content: string }
  | { readonly type: "tool_call_start"; readonly chatId: string; readonly runId: string; readonly seq: number; readonly toolId: string; readonly name: string }
  | { readonly type: "run_terminated"; readonly chatId: string; readonly runId: string; readonly seq: number; readonly outcome: string };

export type ChatHostMessage =
  | { readonly type: "chat/status"; readonly message: string; readonly tone: "info" | "error" }
  | { readonly type: "chat/hostReady"; readonly workspace: string }
  | { readonly type: "chat/context"; readonly items: readonly ChatContextChip[] }
  | { readonly type: "chat/draft"; readonly prompt: string }
  | { readonly type: "chat/run-event"; readonly event: ChatRunEvent };

const MAX_PROMPT_LENGTH = 32_000;

export function parseChatWebviewMessage(value: unknown): ChatWebviewMessage | undefined {
  if (!isObject(value) || typeof value.type !== "string") return undefined;
  if (value.type === "chat/openLogs") return hasOnly(value, ["type"]) ? { type: value.type } : undefined;
  if (value.type === "chat/send" && hasOnly(value, ["type", "prompt"]) && validText(value.prompt, MAX_PROMPT_LENGTH)) {
    return { type: value.type, prompt: value.prompt.trim() };
  }
  if (value.type === "chat/removeContext" && hasOnly(value, ["type", "id"]) && validId(value.id)) {
    return { type: value.type, id: value.id };
  }
  if (value.type === "chat/stop" && hasOnly(value, ["type", "runId"]) && validId(value.runId)) {
    return { type: value.type, runId: value.runId };
  }
  return undefined;
}

export function isChatHostMessage(value: unknown): value is ChatHostMessage {
  if (!isObject(value) || typeof value.type !== "string") return false;
  if (value.type === "chat/status") return hasOnly(value, ["type", "message", "tone"]) && validText(value.message, 1_000) && (value.tone === "info" || value.tone === "error");
  if (value.type === "chat/hostReady") return hasOnly(value, ["type", "workspace"]) && validText(value.workspace, 16_384);
  if (value.type === "chat/context") return hasOnly(value, ["type", "items"]) && Array.isArray(value.items) && value.items.length <= 12 && value.items.every(isContextChip);
  if (value.type === "chat/draft") return hasOnly(value, ["type", "prompt"]) && validText(value.prompt, MAX_PROMPT_LENGTH);
  return value.type === "chat/run-event" && hasOnly(value, ["type", "event"]) && parseChatRunEvent(value.event) !== undefined;
}

/** Converts only the currently implemented Rust host events to UI events. */
export function parseChatRunEvent(value: unknown): ChatRunEvent | undefined {
  if (!isObject(value) || !validId(value.chat_id) || !validId(value.run_id) || !validSequence(value.seq) || !isObject(value.event) || typeof value.event.type !== "string") return undefined;
  const base = { chatId: value.chat_id, runId: value.run_id, seq: value.seq };
  switch (value.event.type) {
    case "run_started":
      return hasOnly(value.event, ["type"]) ? { type: "run_started", ...base } : undefined;
    case "agent_message":
      return hasOnly(value.event, ["type", "role", "content"]) && value.event.role === "assistant" && typeof value.event.content === "string"
        ? { type: "agent_message", ...base, content: value.event.content }
        : undefined;
    case "thinking":
      return hasOnly(value.event, ["type", "content"]) && typeof value.event.content === "string"
        ? { type: "thinking", ...base, content: value.event.content }
        : undefined;
    case "tool_call_start":
      return hasOnly(value.event, ["type", "id", "name"]) && validId(value.event.id) && validText(value.event.name, 512)
        ? { type: "tool_call_start", ...base, toolId: value.event.id, name: value.event.name }
        : undefined;
    case "run_terminated": {
      const outcome = value.event.outcome;
      if (!hasOnly(value.event, ["type", "outcome"]) || !isObject(outcome) || !hasOnly(outcome, ["kind"]) || !validText(outcome.kind, 128)) return undefined;
      return { type: "run_terminated", ...base, outcome: outcome.kind };
    }
    default:
      return undefined;
  }
}

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function hasOnly(value: Record<string, unknown>, keys: readonly string[]): boolean {
  return Object.keys(value).every((key) => keys.includes(key)) && keys.every((key) => key in value);
}

function validText(value: unknown, maxLength: number): value is string {
  return typeof value === "string" && value.trim().length > 0 && value.length <= maxLength;
}

function validId(value: unknown): value is string {
  return validText(value, 512);
}

function validSequence(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value > 0;
}

function isContextChip(value: unknown): value is ChatContextChip {
  if (!isObject(value) || (!hasOnly(value, ["id", "kind", "label", "uri", "range"]) && !hasOnly(value, ["id", "kind", "label", "uri"]))) return false;
  if (!validId(value.id) || !validText(value.label, 1_000) || !validText(value.uri, 16_384) || (value.kind !== "selection" && value.kind !== "file" && value.kind !== "diagnostics")) return false;
  if (value.range === undefined) return true;
  if (!isObject(value.range) || !hasOnly(value.range, ["startLine", "endLine"])) return false;
  const startLine = value.range.startLine;
  const endLine = value.range.endLine;
  return validLine(startLine) && validLine(endLine) && endLine >= startLine;
}

function validLine(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value > 0;
}
