/**
 * Pure helpers for wait-shell diagnostic clipboard content (A6.77).
 */

export function formatDiagnosticClipboardText(input: {
  diagnosticCode?: string;
  message?: string;
  recoveryHint?: string;
}): string | null {
  const code = input.diagnosticCode?.trim();
  const message = input.message?.trim();
  const recovery = input.recoveryHint?.trim();
  if (!code && !message && !recovery) {
    return null;
  }
  const lines = [
    code ? `ALTAI diagnostic: ${code}` : null,
    message ? `Message: ${message}` : null,
    recovery ? `Recovery: ${recovery}` : null,
  ].filter((line): line is string => Boolean(line));
  return lines.join("\n");
}
