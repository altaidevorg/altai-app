/**
 * Attention report payloads hosts exchange for status-bar badges (A6.107).
 */

/** Runtime-validate Webview → Extension Host attention reports. */
export function parseAttentionReportParams(value: unknown): number | null {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    return null;
  }
  const count = (value as { count?: unknown }).count;
  if (typeof count !== "number" || !Number.isFinite(count) || count < 0) {
    return null;
  }
  return Math.floor(count);
}

/**
 * Prefer Inbox when the badge is non-zero so the click lands on work that needs
 * attention; overview otherwise.
 */
export function attentionStatusBarCommand(count: number): string {
  const next = Number.isFinite(count) ? Math.max(0, Math.floor(count)) : 0;
  return next > 0 ? "altai.openOperationsInbox" : "altai.openOperations";
}
