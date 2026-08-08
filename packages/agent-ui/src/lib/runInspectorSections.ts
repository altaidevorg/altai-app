/**
 * Pure run-inspector section mappers for flat display messages (A6.68).
 * Hosts glue pending approvals + message arrays; no vscode/Tauri imports.
 */

import { isEditDiffMessage, type EditDiffMessageLike } from "./editDiffMessagePolicy.js";
import { summarizeTodoItems, type TodoItem } from "./todoParse.js";

export type PendingApprovalLike = {
  approvalId: string;
  toolName?: string;
  input?: unknown;
};

export type ApprovalsInspectorItemView = {
  id: string;
  action: string;
  payload: unknown;
};

export type TodosInspectorItemView = {
  id: string;
  title: string;
  status: string;
};

export type InspectorTodosModelView = {
  done: number;
  total: number;
  todos: TodosInspectorItemView[];
};

export type ChangesInspectorItemView = {
  id: string;
  path: string;
  originalContent: string;
  proposedContent: string;
  isNewFile: boolean;
};

export type ActivityInspectorEventView = {
  id: string;
  label: string;
  detail?: string;
  tone?: "default" | "success" | "warning" | "error";
  createdAt: number;
};

export type RunInspectorSectionsModelView = {
  approvals: ApprovalsInspectorItemView[];
  todos: InspectorTodosModelView | null;
  changes: ChangesInspectorItemView[];
  activity: ActivityInspectorEventView[];
};

export type RunInspectorMessageLike = EditDiffMessageLike & {
  id: string;
  content?: string;
  todos?: readonly TodoItem[];
  filePath?: string;
  toolName?: string;
};

/** Map pending tool approvals to ApprovalsInspector items. */
export function mapApprovalsToInspectorItems(
  approvals: readonly PendingApprovalLike[],
): ApprovalsInspectorItemView[] {
  return approvals.map((row) => ({
    id: row.approvalId,
    action: row.toolName || "tool",
    payload: row.input ?? {},
  }));
}

/** Latest tool message that embeds a todo list, if any. */
export function latestTodosFromMessages(
  messages: readonly RunInspectorMessageLike[],
): InspectorTodosModelView | null {
  for (let i = messages.length - 1; i >= 0; i -= 1) {
    const message = messages[i];
    if (!message || message.role !== "tool" || !message.todos?.length) {
      continue;
    }
    const summary = summarizeTodoItems([...message.todos]);
    const todos: TodosInspectorItemView[] = message.todos.map((todo, index) => ({
      id: todo.id?.trim() || `todo-${index}`,
      title: todo.title,
      status: todo.status,
    }));
    return {
      done: summary.done,
      total: summary.total,
      todos,
    };
  }
  return null;
}

/** Map edit_diff tool rows to ChangesInspector queue entries. */
export function changesFromMessages(
  messages: readonly RunInspectorMessageLike[],
): ChangesInspectorItemView[] {
  const items: ChangesInspectorItemView[] = [];
  for (const message of messages) {
    if (!isEditDiffMessage(message)) {
      continue;
    }
    const path =
      message.filePath?.trim() ||
      message.content?.trim() ||
      message.toolName ||
      "change";
    items.push({
      id: message.id,
      path,
      originalContent: message.diffOriginalText ?? "",
      proposedContent: message.diffModifiedText ?? "",
      isNewFile: !(message.diffOriginalText ?? "").trim(),
    });
  }
  return items;
}

/** Compact tool activity timeline (newest last, capped). */
export function activityFromMessages(
  messages: readonly RunInspectorMessageLike[],
  limit = 24,
): ActivityInspectorEventView[] {
  const events: ActivityInspectorEventView[] = [];
  for (const message of messages) {
    if (message.role !== "tool") {
      continue;
    }
    const label = message.toolName?.trim() || "tool";
    const detail =
      message.filePath?.trim() || (message.content ?? "").slice(0, 120);
    events.push({
      id: message.id,
      label,
      ...(detail ? { detail } : {}),
      tone: "default",
      createdAt: 0,
    });
  }
  if (events.length <= limit) {
    return events;
  }
  return events.slice(-limit);
}

export function buildRunInspectorSections(input: {
  approvals: readonly PendingApprovalLike[];
  messages: readonly RunInspectorMessageLike[];
}): RunInspectorSectionsModelView {
  return {
    approvals: mapApprovalsToInspectorItems(input.approvals),
    todos: latestTodosFromMessages(input.messages),
    changes: changesFromMessages(input.messages),
    activity: activityFromMessages(input.messages),
  };
}

export function hasRunInspectorContent(
  model: RunInspectorSectionsModelView,
): boolean {
  return (
    model.approvals.length > 0 ||
    Boolean(model.todos && model.todos.total > 0) ||
    model.changes.length > 0 ||
    model.activity.length > 0
  );
}
