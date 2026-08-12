/**
 * Presentation-only Webview state persisted via host webview getState/setState (A6.131).
 * Must not hold privileged host context or secrets.
 */

export type PersistedHostStatus = {
  status: string;
  message: string;
  extensionVersion: string;
  diagnosticCode?: string;
};

/** Top-level Chat / Operations / Settings surface. */
export type PersistedAltaiSurface = "chat" | "operations" | "settings";

export type PersistedOperationsView =
  | "overview"
  | "work"
  | "runs"
  | "inbox";

export type PersistedWorkHubView = "runs" | "scheduled";

export type PersistedWebviewState = {
  hostStatus?: PersistedHostStatus;
  surface?: PersistedAltaiSurface;
  operationsView?: PersistedOperationsView;
  workHubView?: PersistedWorkHubView;
  /**
   * Last Chat conversation focused for the current preferred root.
   * Prefer `activeChatIdByRoot` for multi-root / workspace switches.
   */
  activeChatId?: string;
  /**
   * Last focused chat id keyed by preferred workspace root URI so history
   * focus does not leak across workspaces.
   */
  activeChatIdByRoot?: Record<string, string>;
  /**
   * Unsent Chat composer text. Presentation-only; never secrets.
   * Capped on parse/write so getState stays small.
   */
  composerDraft?: string;
  /** Last opened Settings section id (general, models, …). */
  settingsSection?: string;
  /**
   * Multi-root Explorer/display preference only — does not rebind agent host roots.
   */
  preferredRootUri?: string;
  /** Last selected composer agent profile id (presentation only). */
  activeAgentId?: string;
};

/** Resolve the focused chat for a workspace root (map first, then legacy). */
export function activeChatIdForRoot(
  state: PersistedWebviewState,
  rootUri: string | undefined | null,
): string | undefined {
  if (rootUri) {
    const mapped = state.activeChatIdByRoot?.[rootUri];
    if (mapped) return mapped;
    if (state.preferredRootUri === rootUri && state.activeChatId) {
      return state.activeChatId;
    }
    return undefined;
  }
  return state.activeChatId;
}

/** Build a patch that stores focus both legacy and per-root. */
export function activeChatFocusPatch(
  rootUri: string | undefined | null,
  chatId: string | undefined | null,
): PersistedWebviewState {
  const id = typeof chatId === "string" ? chatId.trim() : "";
  if (!rootUri) {
    return { activeChatId: id };
  }
  return {
    preferredRootUri: rootUri,
    activeChatId: id,
    activeChatIdByRoot: { [rootUri]: id },
  };
}

/** Max characters retained for the reloadable composer draft. */
export const MAX_COMPOSER_DRAFT_CHARS = 8_000;

/**
 * Normalize unsent composer text for Webview persistence (trim right only so
 * intentional leading whitespace survives; empty becomes undefined).
 */
export function normalizeComposerDraft(
  value: unknown,
  maxChars: number = MAX_COMPOSER_DRAFT_CHARS,
): string | undefined {
  if (typeof value !== "string") {
    return undefined;
  }
  // Keep trailing newlines the user may still be editing; drop pure empty.
  const capped = value.length > maxChars ? value.slice(0, maxChars) : value;
  if (capped.length === 0) {
    return undefined;
  }
  // Reject control-heavy garbage (except common whitespace).
  if (/[\u0000-\u0008\u000B\u000C\u000E-\u001F]/.test(capped)) {
    return undefined;
  }
  return capped;
}

/** Max length for preferred multi-root URI presentation state. */
export const MAX_PREFERRED_ROOT_URI_CHARS = 1_024;

export function normalizePreferredRootUri(
  value: unknown,
  maxChars: number = MAX_PREFERRED_ROOT_URI_CHARS,
): string | undefined {
  if (typeof value !== "string") {
    return undefined;
  }
  const trimmed = value.trim();
  if (!trimmed || trimmed.length > maxChars) {
    return undefined;
  }
  // Presentation URI only; reject control characters.
  if (/[\u0000-\u001F]/.test(trimmed)) {
    return undefined;
  }
  return trimmed;
}

const SURFACES = new Set<PersistedAltaiSurface>([
  "chat",
  "operations",
  "settings",
]);
const OPS_VIEWS = new Set<PersistedOperationsView>([
  "overview",
  "work",
  "runs",
  "inbox",
]);
const WORK_HUB = new Set<PersistedWorkHubView>(["runs", "scheduled"]);

/**
 * Accept an arbitrary getState() blob. Malformed fields are dropped so partial
 * recovery remains possible (e.g. surface survives a bad hostStatus).
 */
export function parsePersistedWebviewState(
  value: unknown,
): PersistedWebviewState {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    return {};
  }

  const record = value as Record<string, unknown>;
  const next: PersistedWebviewState = {};

  if (isPersistedHostStatus(record.hostStatus)) {
    next.hostStatus = record.hostStatus;
  }
  if (
    typeof record.surface === "string" &&
    SURFACES.has(record.surface as PersistedAltaiSurface)
  ) {
    next.surface = record.surface as PersistedAltaiSurface;
  }
  if (
    typeof record.operationsView === "string" &&
    OPS_VIEWS.has(record.operationsView as PersistedOperationsView)
  ) {
    next.operationsView = record.operationsView as PersistedOperationsView;
  }
  if (
    typeof record.workHubView === "string" &&
    WORK_HUB.has(record.workHubView as PersistedWorkHubView)
  ) {
    next.workHubView = record.workHubView as PersistedWorkHubView;
  }
  const preferred = normalizePreferredRootUri(record.preferredRootUri);
  if (preferred !== undefined) {
    next.preferredRootUri = preferred;
  }
  if (
    typeof record.activeChatId === "string" &&
    record.activeChatId.length > 0 &&
    record.activeChatId.length <= 512
  ) {
    next.activeChatId = record.activeChatId;
  }
  const byRoot = parseActiveChatIdByRoot(record.activeChatIdByRoot);
  if (byRoot) {
    next.activeChatIdByRoot = byRoot;
    // Seed legacy field from preferred root when missing.
    if (!next.activeChatId && next.preferredRootUri) {
      const seeded = byRoot[next.preferredRootUri];
      if (seeded) next.activeChatId = seeded;
    }
  } else if (next.activeChatId && next.preferredRootUri) {
    next.activeChatIdByRoot = {
      [next.preferredRootUri]: next.activeChatId,
    };
  }
  const draft = normalizeComposerDraft(record.composerDraft);
  if (draft !== undefined) {
    next.composerDraft = draft;
  }
  if (
    typeof record.settingsSection === "string" &&
    record.settingsSection.length > 0 &&
    record.settingsSection.length <= 64 &&
    /^[a-z][a-z0-9-]*$/.test(record.settingsSection)
  ) {
    next.settingsSection = record.settingsSection;
  }
  if (
    typeof record.activeAgentId === "string" &&
    record.activeAgentId.length > 0 &&
    record.activeAgentId.length <= 128 &&
    /^[a-z0-9:_-]+$/i.test(record.activeAgentId)
  ) {
    next.activeAgentId = record.activeAgentId;
  }

  return next;
}

/** Shallow merge for presentation patches; never invents privileged fields. */
export function mergePersistedWebviewState(
  current: PersistedWebviewState,
  patch: PersistedWebviewState,
): PersistedWebviewState {
  const next: PersistedWebviewState = { ...current, ...patch };
  if (Object.prototype.hasOwnProperty.call(patch, "composerDraft")) {
    const draft = normalizeComposerDraft(patch.composerDraft);
    if (draft === undefined) {
      delete next.composerDraft;
    } else {
      next.composerDraft = draft;
    }
  }
  if (Object.prototype.hasOwnProperty.call(patch, "activeChatId")) {
    const id =
      typeof patch.activeChatId === "string" ? patch.activeChatId.trim() : "";
    if (!id || id.length > 512) {
      delete next.activeChatId;
    } else {
      next.activeChatId = id;
    }
  }
  if (Object.prototype.hasOwnProperty.call(patch, "activeChatIdByRoot")) {
    const merged = mergeActiveChatIdByRoot(
      current.activeChatIdByRoot,
      patch.activeChatIdByRoot,
    );
    if (!merged || Object.keys(merged).length === 0) {
      delete next.activeChatIdByRoot;
    } else {
      next.activeChatIdByRoot = merged;
    }
  }
  if (Object.prototype.hasOwnProperty.call(patch, "preferredRootUri")) {
    const preferred = normalizePreferredRootUri(patch.preferredRootUri);
    if (preferred === undefined) {
      delete next.preferredRootUri;
    } else {
      next.preferredRootUri = preferred;
    }
  }
  if (Object.prototype.hasOwnProperty.call(patch, "settingsSection")) {
    const section =
      typeof patch.settingsSection === "string"
        ? patch.settingsSection.trim()
        : "";
    if (
      !section ||
      section.length > 64 ||
      !/^[a-z][a-z0-9-]*$/.test(section)
    ) {
      delete next.settingsSection;
    } else {
      next.settingsSection = section;
    }
  }
  if (Object.prototype.hasOwnProperty.call(patch, "activeAgentId")) {
    const id =
      typeof patch.activeAgentId === "string" ? patch.activeAgentId.trim() : "";
    if (!id || id.length > 128 || !/^[a-z0-9:_-]+$/i.test(id)) {
      delete next.activeAgentId;
    } else {
      next.activeAgentId = id;
    }
  }
  return next;
}

function parseActiveChatIdByRoot(
  value: unknown,
): Record<string, string> | undefined {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    return undefined;
  }
  const next: Record<string, string> = {};
  for (const [root, chatId] of Object.entries(
    value as Record<string, unknown>,
  )) {
    const preferred = normalizePreferredRootUri(root);
    if (
      !preferred ||
      typeof chatId !== "string" ||
      !chatId ||
      chatId.length > 512
    ) {
      continue;
    }
    next[preferred] = chatId;
  }
  return Object.keys(next).length > 0 ? next : undefined;
}

function mergeActiveChatIdByRoot(
  current: Record<string, string> | undefined,
  patch: Record<string, string> | undefined,
): Record<string, string> | undefined {
  if (!patch) {
    return current;
  }
  const next: Record<string, string> = { ...(current ?? {}) };
  for (const [root, chatId] of Object.entries(patch)) {
    const preferred = normalizePreferredRootUri(root);
    if (!preferred) continue;
    const id = typeof chatId === "string" ? chatId.trim() : "";
    if (!id || id.length > 512) {
      delete next[preferred];
    } else {
      next[preferred] = id;
    }
  }
  return next;
}

function isPersistedHostStatus(value: unknown): value is PersistedHostStatus {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    return false;
  }
  const record = value as Record<string, unknown>;
  if (
    typeof record.status !== "string" ||
    typeof record.message !== "string" ||
    typeof record.extensionVersion !== "string"
  ) {
    return false;
  }
  if (
    Object.prototype.hasOwnProperty.call(record, "diagnosticCode") &&
    typeof record.diagnosticCode !== "string"
  ) {
    return false;
  }
  return true;
}
