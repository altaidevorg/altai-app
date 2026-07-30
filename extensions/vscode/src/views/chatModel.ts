import type { ChatContextChip, ChatHostMessage, ChatRunEvent } from "./chatMessages.js";

export type ChatEntry = {
  readonly id: string;
  readonly kind: "user" | "assistant" | "thinking" | "tool" | "terminal";
  readonly content: string;
};

export type ChatState = {
  readonly entries: readonly ChatEntry[];
  readonly contextItems: readonly ChatContextChip[];
  readonly draft?: string;
  readonly status: string;
  readonly statusTone: "info" | "error";
  readonly awaitingRun?: boolean;
  readonly activeChatId?: string;
  readonly activeRunId?: string;
  readonly lastSeq?: number;
};

export type ChatAction =
  | { readonly type: "promptSent"; readonly prompt: string }
  | { readonly type: "hostMessage"; readonly message: ChatHostMessage };

export const initialChatState: ChatState = {
  entries: [],
  contextItems: [],
  status: "Ready. Session history, replay, and steering are not available in this MVP.",
  statusTone: "info",
};

export function reduceChat(state: ChatState, action: ChatAction): ChatState {
  if (action.type === "promptSent") {
    return {
      ...state,
      draft: undefined,
      entries: [...state.entries, entry("user", action.prompt)],
      status: "Starting ALTAI…",
      statusTone: "info",
      awaitingRun: true,
    };
  }
  const message = action.message;
  if (message.type === "chat/status") return { ...state, status: message.message, statusTone: message.tone, ...(message.tone === "error" ? { awaitingRun: false } : {}) };
  if (message.type === "chat/hostReady") return { ...state, status: `Connected to ${message.workspace}`, statusTone: "info" };
  if (message.type === "chat/context") return { ...state, contextItems: message.items };
  if (message.type === "chat/draft") return { ...state, draft: message.prompt };
  return reduceRunEvent(state, message.event);
}

function reduceRunEvent(state: ChatState, event: ChatRunEvent): ChatState {
  if (event.type === "run_started") {
    return { ...state, activeChatId: event.chatId, activeRunId: event.runId, awaitingRun: false, lastSeq: event.seq, status: "ALTAI is working…", statusTone: "info" };
  }
  if (state.activeChatId !== event.chatId || state.activeRunId !== event.runId || (state.lastSeq !== undefined && event.seq <= state.lastSeq)) return state;
  const next = { ...state, lastSeq: event.seq };
  switch (event.type) {
    case "agent_message":
      return { ...next, entries: [...next.entries, entry("assistant", event.content)] };
    case "thinking":
      return { ...next, entries: [...next.entries, entry("thinking", event.content)] };
    case "tool_call_start":
      return { ...next, entries: [...next.entries, entry("tool", `Using ${event.name}`)] };
    case "run_terminated":
      return {
        ...next,
        entries: [...next.entries, entry("terminal", terminalText(event.outcome))],
        activeChatId: undefined,
        activeRunId: undefined,
        awaitingRun: false,
        lastSeq: undefined,
        status: event.outcome === "completed" ? "Ready." : `Run ${event.outcome}.`,
        statusTone: event.outcome === "completed" ? "info" : "error",
      };
    default:
      return next;
  }
}

function entry(kind: ChatEntry["kind"], content: string): ChatEntry {
  return { id: `${kind}-${Date.now()}-${Math.random().toString(36).slice(2)}`, kind, content };
}

function terminalText(outcome: string): string {
  return outcome === "completed" ? "Run completed." : outcome === "cancelled" ? "Run cancelled." : `Run ${outcome}.`;
}
