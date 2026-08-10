/**
 * Pure selected-context XML block + prompt assembly (A6.230).
 */

const DEFAULT_FILE_MAX = 60_000;
const DEFAULT_TERMINAL_MAX = 60_000;
const DEFAULT_DIFF_MAX = 80_000;

/** Wrap a file body in a context-file tag (content capped). */
export function wrapContextFileBlock(
  path: string,
  content: string,
  maxChars: number = DEFAULT_FILE_MAX,
): string {
  return `<context-file path="${path}">\n${content.slice(0, maxChars)}\n</context-file>`;
}

/** Wrap terminal output; null when empty after trim. */
export function wrapTerminalContextBlock(
  output: string,
  maxChars: number = DEFAULT_TERMINAL_MAX,
): string | null {
  const trimmed = output.trim();
  if (!trimmed) return null;
  return `<terminal-context>\n${trimmed.slice(0, maxChars)}\n</terminal-context>`;
}

/** Wrap working-tree diff; null when empty after trim. */
export function wrapWorkingTreeDiffBlock(
  diffText: string,
  truncated = false,
  maxChars: number = DEFAULT_DIFF_MAX,
): string | null {
  const trimmed = diffText.trim();
  if (!trimmed) return null;
  const attr = truncated ? ' truncated="true"' : "";
  return `<working-tree-diff${attr}>\n${trimmed.slice(0, maxChars)}\n</working-tree-diff>`;
}

/**
 * Append assembled context blocks under `<selected-context>`.
 * When there are no blocks, returns the original prompt (untrimmed).
 */
export function composePromptWithSelectedContext(
  prompt: string,
  blocks: readonly string[],
): string {
  if (blocks.length === 0) return prompt;
  return `${prompt.trim()}\n\n<selected-context>\n${blocks.join("\n\n")}\n</selected-context>`;
}
