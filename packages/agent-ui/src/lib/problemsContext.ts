/**
 * Pure formatter for language diagnostics → composer selection text (A6.117).
 */

export type DiagnosticLike = {
  severity: number;
  message: string;
  startLine: number;
  startCharacter: number;
  endLine: number;
  endCharacter: number;
  source?: string;
};

export type FileDiagnosticsBundle = {
  pathLabel: string;
  diagnostics: readonly DiagnosticLike[];
};

const SEVERITY_LABEL: Record<number, string> = {
  0: "Error",
  1: "Warning",
  2: "Info",
  3: "Hint",
};

const MAX_PROBLEMS = 40;
const MAX_CHARS = 12_000;
const MAX_FILES = 12;

export function formatProblemsContextText(
  pathLabel: string,
  diagnostics: readonly DiagnosticLike[],
): string | null {
  return formatProblemsBundles([{ pathLabel, diagnostics }]);
}

/**
 * Format one or more files' diagnostics into a single attach payload body.
 */
export function formatProblemsBundles(
  files: readonly FileDiagnosticsBundle[],
): string | null {
  const nonEmpty = files.filter((file) => file.diagnostics.length > 0);
  if (nonEmpty.length === 0) {
    return null;
  }

  const lines: string[] = [];
  let total = 0;
  let filesUsed = 0;

  for (const file of nonEmpty) {
    if (filesUsed >= MAX_FILES || total >= MAX_PROBLEMS) {
      break;
    }
    const sorted = [...file.diagnostics].sort((a, b) => a.severity - b.severity);
    lines.push(`Problems in ${file.pathLabel}:`);
    filesUsed += 1;
    for (const d of sorted) {
      if (total >= MAX_PROBLEMS) {
        lines.push(`…and more (capped at ${MAX_PROBLEMS})`);
        break;
      }
      const label = SEVERITY_LABEL[d.severity] ?? `Severity ${d.severity}`;
      const src = d.source?.trim() ? ` [${d.source.trim()}]` : "";
      const range = `L${d.startLine + 1}:${d.startCharacter + 1}`;
      lines.push(`- ${label}${src} ${range}: ${d.message.trim()}`);
      total += 1;
    }
    lines.push("");
  }

  if (total === 0) {
    return null;
  }

  let text = lines.join("\n").trimEnd();
  if (text.length > MAX_CHARS) {
    text = `${text.slice(0, MAX_CHARS - 1)}…`;
  }
  return text;
}
