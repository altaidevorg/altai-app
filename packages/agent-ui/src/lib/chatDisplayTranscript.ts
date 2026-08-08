/**
 * Pure chat transcript display model (host-neutral). Builds structured
 * bubbles from session history + live AgentEvent stream.
 * Wave 4 / A6.31 — shared by VS Code ChatMessageList adapters and future
 * ports-first AiChatView paths.
 */

import type { AgentEvent } from "@altai/host-contract";
import type { ContextChip } from "../components/ContextChips.js";
import type { TodoItem } from "./todoParse.js";
import {
  isTodoToolName,
  parseTodoItemsFromInput,
  summarizeTodoItems,
} from "./todoParse.js";
import { looksLikePath, pathToFileUri, toolBubbleContent } from "./chatHref.js";
import { renumberUserTurnIds } from "./chatMessageEdit.js";
import { prepareUserTurnDisplay } from "./userTurnDisplay.js";

export type ChatDisplayRole =
  | "user"
  | "assistant"
  | "system"
  | "tool"
  | "meta";

export type ChatDisplayMessage = {
  id: string;
  role: ChatDisplayRole;
  content: string;
  /** Live stream still appending to this bubble. */
  streaming?: boolean;
  /** Optional context chips from attached editor / terminal / diff. */
  chips?: ContextChip[];
  /** Slash command name from leading altai-command marker (session restore). */
  commandName?: string;
  /** Tool name when role is tool. */
  toolName?: string;
  /** Workspace URI to open (file tools). */
  fileUri?: string;
  /** Display path when a tool targets a file. */
  filePath?: string;
  /** Original text for edit_diff rows (host opens via workspace.openDiff). */
  diffOriginalText?: string;
  /** Proposed text for edit_diff rows. */
  diffModifiedText?: string;
  /** Parsed todo_write / task list items for shared TodoChecklist. */
  todos?: TodoItem[];
};

export type SessionMessageLike = {
  id?: string;
  role: "user" | "assistant" | "system" | "tool";
  content: string;
};

const DEFAULT_MAX = 200;

export function newDisplayMessageId(prefix: string): string {
  return `${prefix}_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 9)}`;
}

/**
 * Map durable session messages into display bubbles (user/assistant only).
 * User turns strip altai-command markers + context XML into chips (A6.28).
 */
export function displayMessagesFromSession(
  messages: readonly SessionMessageLike[],
  options?: { maxMessages?: number },
): ChatDisplayMessage[] {
  const max = options?.maxMessages ?? DEFAULT_MAX;
  const mapped = messages
    .filter(
      (message) => message.role === "user" || message.role === "assistant",
    )
    .slice(-max)
    .map((message) => {
      if (message.role === "user") {
        const prepared = prepareUserTurnDisplay(message.content);
        const body = prepared.text.replace(/\s+/g, " ").trim();
        const hasChrome =
          Boolean(prepared.commandName) || prepared.chips.length > 0;
        return {
          id: message.id?.trim() || newDisplayMessageId(message.role),
          role: "user" as const,
          content: body || (hasChrome ? "" : "(empty)"),
          ...(prepared.commandName
            ? { commandName: prepared.commandName }
            : {}),
          ...(prepared.chips.length > 0 ? { chips: prepared.chips } : {}),
        };
      }
      const content = message.content.replace(/\s+/g, " ").trim() || "(empty)";
      return {
        id: message.id?.trim() || newDisplayMessageId(message.role),
        role: message.role,
        content,
      };
    });
  return renumberUserTurnIds(mapped);
}

/** Empty home when no conversational content is visible. */
export function shouldShowChatEmptyHome(
  messages: readonly ChatDisplayMessage[],
): boolean {
  return !messages.some(
    (message) =>
      (message.role === "user" || message.role === "assistant") &&
      message.content.trim().length > 0,
  );
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/**
 * Extract assistant/user text from a mapped AgentEvent payload.
 * Handles nested crate shapes (`content`, `text`, `delta`, nested type).
 */
export function textFromAgentEvent(event: AgentEvent): {
  role: "user" | "assistant";
  text: string;
  done: boolean;
} | null {
  const body = isRecord(event.payload) ? event.payload : {};
  const nested =
    isRecord(body.event) ? body.event : isRecord(body.payload) ? body.payload : body;
  const crateType =
    typeof nested.type === "string"
      ? nested.type
      : typeof body.type === "string"
        ? body.type
        : event.type;

  if (
    crateType === "agent_message" ||
    crateType === "message" ||
    event.type === "message"
  ) {
    const text =
      (typeof nested.content === "string" && nested.content) ||
      (typeof nested.text === "string" && nested.text) ||
      (typeof nested.delta === "string" && nested.delta) ||
      (typeof body.content === "string" && body.content) ||
      (typeof body.text === "string" && body.text) ||
      "";
    if (!text) {
      return null;
    }
    const roleRaw =
      (typeof nested.role === "string" && nested.role) ||
      (typeof body.role === "string" && body.role) ||
      "assistant";
    const role = roleRaw === "user" ? "user" : "assistant";
    const done = nested.done === true || body.done === true;
    return { role, text, done };
  }

  if (crateType === "thinking" || event.type === "reasoning") {
    // Reasoning is available but we hide noisy intermediate thinking by default.
    return null;
  }

  return null;
}

/**
 * Best-effort workspace open target for a tool event payload (Desktop tools
 * often put path on the crate event or under `input`).
 */
export function extractToolFileTarget(payload: unknown): {
  path?: string;
  uri?: string;
} {
  const stack: unknown[] = [payload];
  const visited = new Set<unknown>();
  let path: string | undefined;
  let uri: string | undefined;

  while (stack.length > 0 && (!path || !uri)) {
    const cur = stack.pop();
    if (!cur || visited.has(cur) || !isRecord(cur)) {
      continue;
    }
    visited.add(cur);
    if (!uri && typeof cur.uri === "string" && cur.uri.trim()) {
      uri = cur.uri.trim();
    }
    if (!path) {
      for (const key of [
        "path",
        "file_path",
        "filePath",
        "target",
        "file",
      ] as const) {
        const value = cur[key];
        if (typeof value === "string" && value.trim() && looksLikePath(value)) {
          path = value.trim();
          break;
        }
      }
    }
    for (const nestedKey of ["input", "args", "params", "event", "payload"]) {
      if (cur[nestedKey] !== undefined) {
        stack.push(cur[nestedKey]);
      }
    }
  }

  return {
    ...(path ? { path } : {}),
    ...(uri ? { uri } : !path ? {} : { uri: pathToFileUri(path) }),
  };
}

export function extractTodoToolItems(payload: unknown): TodoItem[] {
  if (!isRecord(payload)) {
    return [];
  }
  const candidates: unknown[] = [
    payload.input,
    payload.args,
    payload.params,
    payload,
  ];
  if (isRecord(payload.event)) {
    candidates.unshift(
      payload.event.input,
      payload.event.args,
      payload.event,
    );
  }
  for (const candidate of candidates) {
    const items = parseTodoItemsFromInput(candidate);
    if (items.length > 0) {
      return items;
    }
  }
  return [];
}

/**
 * Normalize native `edit_diff` crates (file + before/after) for Open Diff.
 */
export function extractEditDiff(payload: unknown): {
  path: string;
  originalText: string;
  modifiedText: string;
  hunkId?: string;
} | null {
  const candidates: unknown[] = [payload];
  if (isRecord(payload)) {
    if (payload.event !== undefined) {
      candidates.push(payload.event);
    }
    if (payload.edit_diff !== undefined) {
      candidates.push(payload.edit_diff);
    }
    if (payload.payload !== undefined) {
      candidates.push(payload.payload);
    }
  }
  for (const candidate of candidates) {
    if (!isRecord(candidate)) {
      continue;
    }
    const path =
      (typeof candidate.file === "string" && candidate.file.trim()) ||
      (typeof candidate.path === "string" && candidate.path.trim()) ||
      "";
    const originalText =
      typeof candidate.before === "string"
        ? candidate.before
        : typeof candidate.original === "string"
          ? candidate.original
          : typeof candidate.originalText === "string"
            ? candidate.originalText
            : null;
    const modifiedText =
      typeof candidate.after === "string"
        ? candidate.after
        : typeof candidate.modified === "string"
          ? candidate.modified
          : typeof candidate.proposed === "string"
            ? candidate.proposed
            : typeof candidate.modifiedText === "string"
              ? candidate.modifiedText
              : null;
    if (!path || originalText === null || modifiedText === null) {
      continue;
    }
    return {
      path,
      originalText,
      modifiedText,
      ...(typeof candidate.hunk_id === "string"
        ? { hunkId: candidate.hunk_id }
        : typeof candidate.hunkId === "string"
          ? { hunkId: candidate.hunkId }
          : {}),
    };
  }
  return null;
}

function pushTrimmed(
  messages: ChatDisplayMessage[],
  next: ChatDisplayMessage,
  max: number,
): ChatDisplayMessage[] {
  return [...messages, next].slice(-max);
}

/**
 * Apply a live host event into the structured transcript.
 * Ignores events for other chats when activeChatId is set.
 */
export function applyAgentEventToMessages(
  messages: readonly ChatDisplayMessage[],
  event: AgentEvent,
  options: {
    activeChatId: string | null;
    maxMessages?: number;
  },
): ChatDisplayMessage[] {
  const active = options.activeChatId;
  if (active && event.chatId && event.chatId !== active) {
    return [...messages];
  }
  const max = options.maxMessages ?? DEFAULT_MAX;
  const list = [...messages];

  const textEvent = textFromAgentEvent(event);
  if (textEvent) {
    const last = list[list.length - 1];
    if (
      last &&
      last.role === textEvent.role &&
      last.streaming &&
      textEvent.role === "assistant"
    ) {
      // Snapshot (replacement) when the payload re-sends the full buffer;
      // otherwise append as a delta.
      const nextContent = textEvent.text.startsWith(last.content)
        ? textEvent.text
        : last.content + textEvent.text;
      const next = [...list];
      next[next.length - 1] = {
        ...last,
        content: nextContent,
        streaming: !textEvent.done,
      };
      return next.slice(-max);
    }
    return pushTrimmed(
      list,
      {
        id: `live_${event.seq}_${event.runId}`,
        role: textEvent.role,
        content: textEvent.text,
        streaming: textEvent.role === "assistant" && !textEvent.done,
      },
      max,
    );
  }

  // Lifecycle / tool meta (low noise).
  const body = isRecord(event.payload) ? event.payload : {};
  const crateType =
    (isRecord(body.event) && typeof body.event.type === "string" && body.event.type) ||
    (typeof body.type === "string" && body.type) ||
    event.type;

  if (
    crateType === "run_terminated" ||
    (event.type === "lifecycle" &&
      (crateType === "run_terminated" || body.outcome !== undefined))
  ) {
    return list.map((message) =>
      message.streaming ? { ...message, streaming: false } : message,
    );
  }

  if (crateType === "edit_diff" || event.type === "diff") {
    const diff = extractEditDiff(event.payload);
    if (diff) {
      const base = diff.path.replace(/\\/g, "/").split("/").pop() || diff.path;
      return pushTrimmed(
        list,
        {
          id: newDisplayMessageId("diff"),
          role: "tool",
          content: `Edit · ${base}`,
          toolName: "edit_diff",
          filePath: diff.path,
          fileUri: pathToFileUri(diff.path),
          diffOriginalText: diff.originalText,
          diffModifiedText: diff.modifiedText,
        },
        max,
      );
    }
  }

  if (
    crateType === "tool_call_start" ||
    (event.type === "tool" && crateType !== "edit_diff")
  ) {
    const name =
      (typeof body.name === "string" && body.name) ||
      (isRecord(body.event) &&
        typeof body.event.name === "string" &&
        body.event.name) ||
      (isRecord(body.event) &&
        typeof body.event.tool === "string" &&
        body.event.tool) ||
      (typeof body.tool === "string" && body.tool) ||
      "tool";
    const file = extractToolFileTarget(event.payload);
    const todos = isTodoToolName(name)
      ? extractTodoToolItems(event.payload)
      : [];
    const todoSummary =
      todos.length > 0 ? summarizeTodoItems(todos) : null;
    const content =
      todoSummary && isTodoToolName(name)
        ? `Todos · ${todoSummary.done}/${todoSummary.total} done`
        : toolBubbleContent(name, file.path);
    return pushTrimmed(
      list,
      {
        id: newDisplayMessageId("tool"),
        role: "tool",
        content,
        toolName: name,
        ...(file.uri ? { fileUri: file.uri } : {}),
        ...(file.path ? { filePath: file.path } : {}),
        ...(todos.length > 0 ? { todos } : {}),
      },
      max,
    );
  }

  if (event.type === "warning") {
    const warn =
      (typeof body.warning === "string" && body.warning) ||
      (typeof body.message === "string" && body.message) ||
      "Run warning";
    return pushTrimmed(
      list,
      {
        id: newDisplayMessageId("meta"),
        role: "meta",
        content: warn,
      },
      max,
    );
  }

  return list;
}

export function appendUserMessage(
  messages: readonly ChatDisplayMessage[],
  content: string,
  options?: { maxMessages?: number; chips?: ContextChip[] },
): ChatDisplayMessage[] {
  const max = options?.maxMessages ?? DEFAULT_MAX;
  const withUser = pushTrimmed(
    [...messages],
    {
      id: newDisplayMessageId("user"),
      role: "user",
      content: content.trim(),
      ...(options?.chips && options.chips.length > 0
        ? { chips: options.chips }
        : {}),
    },
    max,
  );
  return renumberUserTurnIds(withUser);
}

export function appendMetaMessage(
  messages: readonly ChatDisplayMessage[],
  content: string,
  options?: { maxMessages?: number },
): ChatDisplayMessage[] {
  const max = options?.maxMessages ?? DEFAULT_MAX;
  return pushTrimmed(
    [...messages],
    {
      id: newDisplayMessageId("meta"),
      role: "meta",
      content,
    },
    max,
  );
}
