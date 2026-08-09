/**
 * Pure todo_write item normalize (A6.163).
 * Host owns store/apply; package maps free-form tool input rows.
 */

import { normalizeTodoStatus, type SharedTodoStatus } from "./todoStatus.js";

export type ParsedTodoWriteItem = {
  id: string;
  title: string;
  description?: string;
  status: SharedTodoStatus;
};

/**
 * Map raw `todo_write` item records into host-ready todo rows.
 * Field names vary across agent runtimes; each field is read defensively.
 */
export function parseTodoWriteItems(
  items: readonly Record<string, unknown>[],
  sessionId: string,
): ParsedTodoWriteItem[] {
  return items.map((it, i) => {
    const title =
      (typeof it.content === "string" && it.content) ||
      (typeof it.title === "string" && it.title) ||
      (typeof it.task === "string" && it.task) ||
      (typeof it.text === "string" && it.text) ||
      "Untitled task";
    const id = typeof it.id === "string" ? it.id : `${sessionId}:${i}`;
    const description =
      typeof it.description === "string" ? it.description : undefined;
    return {
      id,
      title,
      status: normalizeTodoStatus(it.status),
      ...(description !== undefined ? { description } : {}),
    };
  });
}
