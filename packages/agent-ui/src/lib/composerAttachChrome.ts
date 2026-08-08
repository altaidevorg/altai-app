/**
 * Pure builders: turn host editor/git/terminal state into composer context
 * chips (A6.99). No host I/O or secrets.
 */

import {
  formatGitDiffSummary,
  formatTerminalAttachText,
  type GitDiffFileLine,
} from "./attachFormatChrome.js";
import {
  basenamePath,
  countLines,
  newContextItemId,
  type ComposerContextItem,
} from "./composerContextChrome.js";

export type { GitDiffFileLine };

/**
 * Build an active-file context chip (URI only; contents stay on host).
 */
export function buildFileContextItem(
  file: { uri: string; path: string } | null | undefined,
): Extract<ComposerContextItem, { kind: "file" }> | null {
  const uri = file?.uri?.trim() ?? "";
  const path = file?.path?.trim() ?? "";
  if (!uri || !path) {
    return null;
  }
  return {
    id: newContextItemId("file"),
    kind: "file",
    uri,
    name: basenamePath(path),
    path,
  };
}

/**
 * Build a selection context chip from editor selection text.
 */
export function buildSelectionContextItem(
  selection:
    | {
        uri: string;
        path: string;
        text: string;
      }
    | null
    | undefined,
): Extract<ComposerContextItem, { kind: "selection" }> | null {
  const text = selection?.text ?? "";
  if (!text.trim()) {
    return null;
  }
  const uri = selection?.uri?.trim() ?? "";
  const path = selection?.path?.trim() || "selection";
  return {
    id: newContextItemId("selection"),
    kind: "selection",
    ...(uri ? { uri } : {}),
    path,
    text,
    lines: countLines(text),
  };
}

/**
 * Build a diff context chip from host git context (patch or path/status summary).
 */
export function buildDiffContextItem(
  diff:
    | {
        branch?: string;
        patch?: string;
        files?: readonly GitDiffFileLine[];
      }
    | null
    | undefined,
): Extract<ComposerContextItem, { kind: "diff" }> | null {
  const patch =
    diff?.patch?.trim() ||
    formatGitDiffSummary({
      ...(diff?.branch ? { branch: diff.branch } : {}),
      files: diff?.files ?? [],
    }) ||
    "";
  if (!patch) {
    return null;
  }
  const name = diff?.branch ? `diff · ${diff.branch}` : "Working tree diff";
  return {
    id: newContextItemId("diff"),
    kind: "diff",
    name,
    text: patch,
    lines: countLines(patch),
  };
}

/**
 * Build a terminal context chip from host terminal context.
 */
export function buildTerminalContextItem(
  terminal:
    | {
        cwd?: string;
        selectedText?: string;
        lastCommand?: string;
      }
    | null
    | undefined,
): Extract<ComposerContextItem, { kind: "terminal" }> | null {
  const text = formatTerminalAttachText({
    ...(terminal?.selectedText !== undefined
      ? { selectedText: terminal.selectedText }
      : {}),
    ...(terminal?.lastCommand !== undefined
      ? { lastCommand: terminal.lastCommand }
      : {}),
    ...(terminal?.cwd !== undefined ? { cwd: terminal.cwd } : {}),
  });
  if (!text) {
    return null;
  }
  return {
    id: newContextItemId("terminal"),
    name: terminal?.cwd ? basenamePath(terminal.cwd) : "Terminal",
    kind: "terminal",
    text,
    lines: countLines(text),
  };
}
