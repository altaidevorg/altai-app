/**
 * Pure helpers for plan-mode strip and sticky todo summary chrome (A6.93).
 */

import type { PermissionMode } from "@altai/host-contract";
import type { TodoItem } from "./todoParse.js";

/** Plan mode is the settings permission mode, not a separate HostPorts surface. */
export function isPlanPermissionMode(
  mode: PermissionMode | null | undefined,
): boolean {
  return mode === "plan";
}

/**
 * Prefer the most recent tool message that carries todos (todo_write).
 * Empty when no session tool has written a checklist yet.
 *
 * Named distinctly from `latestTodosFromMessages` in runInspectorSections
 * (which returns the inspector model view, not raw TodoItem rows).
 */
export function latestTodoItemsFromDisplayMessages(
  messages: readonly { todos?: readonly TodoItem[] | undefined }[],
): TodoItem[] {
  for (let i = messages.length - 1; i >= 0; i -= 1) {
    const message = messages[i];
    if (message?.todos && message.todos.length > 0) {
      return [...message.todos];
    }
  }
  return [];
}

/** Sensible mode after leaving read-only plan mode. */
export function permissionModeAfterExitPlan(): PermissionMode {
  return "auto-edit";
}
