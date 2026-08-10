/**
 * Pure Paper import + Compact now labels (A6.268).
 * Closes the Desktop AI pure-string micro-extract drain for common chrome.
 */

export const PAPER_URL_ARIA_LABEL = "Paper URL";

export const PAPER_URL_PLACEHOLDER =
  "Paste arXiv URL (e.g. arxiv.org/abs/2301.12345)";

export function paperImportSubmitLabel(fetching: boolean): string {
  return fetching ? "Fetching..." : "Fetch";
}

export const COMPACT_NOW_TITLE = "Compact context (run /compact now)";
