/**
 * Pure helpers for Operations deep-link payloads (A6.124).
 * Shared by Extension Host commands and the Webview (no React / vscode APIs).
 */

export type OperationsDeepLinkView = "overview" | "work" | "runs" | "inbox";
export type OperationsDeepLinkWorkHubView = "runs" | "scheduled";

export type OpenOperationsPayload = {
  key: number;
  view: OperationsDeepLinkView;
  workHubView?: OperationsDeepLinkWorkHubView;
  composeTask?: boolean;
  composeAutomation?: boolean;
  draftTitle?: string;
};

const VIEWS = new Set<OperationsDeepLinkView>([
  "overview",
  "work",
  "runs",
  "inbox",
]);

const WORK_HUB = new Set<OperationsDeepLinkWorkHubView>(["runs", "scheduled"]);

/**
 * Runtime-validate an `operations.open` event payload.
 * Returns null when the payload is malformed.
 */
export function parseOpenOperationsPayload(
  value: unknown,
): OpenOperationsPayload | null {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    return null;
  }
  const record = value as Record<string, unknown>;
  if (typeof record.key !== "number" || !Number.isFinite(record.key)) {
    return null;
  }
  if (
    typeof record.view !== "string" ||
    !VIEWS.has(record.view as OperationsDeepLinkView)
  ) {
    return null;
  }
  const payload: OpenOperationsPayload = {
    key: record.key,
    view: record.view as OperationsDeepLinkView,
  };
  if (record.workHubView !== undefined) {
    if (
      typeof record.workHubView !== "string" ||
      !WORK_HUB.has(record.workHubView as OperationsDeepLinkWorkHubView)
    ) {
      return null;
    }
    payload.workHubView = record.workHubView as OperationsDeepLinkWorkHubView;
  }
  if (record.composeTask !== undefined) {
    if (typeof record.composeTask !== "boolean") {
      return null;
    }
    payload.composeTask = record.composeTask;
  }
  if (record.composeAutomation !== undefined) {
    if (typeof record.composeAutomation !== "boolean") {
      return null;
    }
    payload.composeAutomation = record.composeAutomation;
  }
  if (record.draftTitle !== undefined) {
    if (typeof record.draftTitle !== "string") {
      return null;
    }
    payload.draftTitle = record.draftTitle;
  }
  return payload;
}

export function buildOpenOperationsPayload(input: {
  view?: OperationsDeepLinkView;
  workHubView?: OperationsDeepLinkWorkHubView;
  key?: number;
  composeTask?: boolean;
  composeAutomation?: boolean;
  draftTitle?: string;
}): OpenOperationsPayload {
  const payload: OpenOperationsPayload = {
    key: input.key ?? Date.now(),
    view: input.view ?? "overview",
  };
  if (input.workHubView) {
    payload.workHubView = input.workHubView;
  }
  if (input.composeTask) {
    payload.composeTask = true;
  }
  if (input.composeAutomation) {
    payload.composeAutomation = true;
  }
  if (input.draftTitle !== undefined && input.draftTitle.length > 0) {
    payload.draftTitle = input.draftTitle;
  }
  return payload;
}
