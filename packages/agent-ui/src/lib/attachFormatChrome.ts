/**
 * Pure presentation of git change lists (path/status only) (A6.94).
 * Hosts and Webview attach helpers share this. Never includes file content
 * and never spawns `git`.
 */

export type GitDiffFileLine = {
  path: string;
  status: string;
};

export function formatGitDiffSummary(input: {
  branch?: string;
  files: readonly GitDiffFileLine[];
}): string | null {
  if (input.files.length === 0) {
    return null;
  }
  const head = input.branch?.trim()
    ? `Working tree changes on ${input.branch.trim()}`
    : "Working tree changes";
  const lines = input.files
    .map((file) => {
      const path = file.path.trim();
      const status = file.status.trim();
      if (!path) {
        return null;
      }
      return status ? `- ${status}  ${path}` : `- ${path}`;
    })
    .filter((line): line is string => Boolean(line));
  if (lines.length === 0) {
    return null;
  }
  return [`${head}:`, ...lines].join("\n");
}

/**
 * Pure helper: turn terminal context into attachable presentation text.
 * Attach Terminal and Ask About Terminal stay consistent. No host imports.
 */
export function formatTerminalAttachText(input: {
  selectedText?: string | null;
  lastCommand?: string | null;
  cwd?: string | null;
}): string | null {
  const selection = input.selectedText?.trim();
  if (selection) {
    return selection;
  }
  const command = input.lastCommand?.trim();
  if (command) {
    return command;
  }
  const cwd = input.cwd?.trim();
  if (cwd) {
    return `Active terminal cwd: ${cwd}`;
  }
  return null;
}
