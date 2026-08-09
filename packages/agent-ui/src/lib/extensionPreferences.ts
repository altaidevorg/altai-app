/**
 * Allowlisted host configuration values for Studio preferences (A6.132).
 * Mirrors Desktop Studio preferences that apply to VS Code host surface.
 * Secrets never live here.
 */

export const EXTENSION_SETTING_KEYS = [
  // Core / general
  "agentHostPath",
  "openPanelOnStartup",
  "autoFocusComposer",
  "showFollowupHints",
  "rememberPermissionMode",
  "agentPickerEnabled",
  "bypassPermissionsEnabled",
  // Context (UI + future host wiring)
  "compactionThresholdPercent",
  "compactionThresholdTokens",
  "compactionTailTurns",
  "compactionPrune",
  "compactionPruneRecencyTokens",
  // Agents
  "customInstructions",
  "snippetsJson",
  // Accessibility (webview chrome)
  "reduceMotion",
  "highContrast",
  "largerText",
  "underlineLinks",
  "focusRing",
  "chatAnnounce",
  "approvalAnnounceAssertive",
  "showSkipLinks",
] as const;

export type ExtensionSettingKey = (typeof EXTENSION_SETTING_KEYS)[number];

export type ReduceMotionPref = "system" | "always" | "never";
export type FocusRingPref = "default" | "strong";
export type ChatAnnouncePref = "off" | "polite" | "assertive";

export type ExtensionPreferences = {
  agentHostPath: string;
  openPanelOnStartup: boolean;
  autoFocusComposer: boolean;
  showFollowupHints: boolean;
  rememberPermissionMode: boolean;
  agentPickerEnabled: boolean;
  bypassPermissionsEnabled: boolean;
  compactionThresholdPercent: number | null;
  compactionThresholdTokens: number;
  compactionTailTurns: number;
  compactionPrune: boolean;
  compactionPruneRecencyTokens: number;
  customInstructions: string;
  snippetsJson: string;
  reduceMotion: ReduceMotionPref;
  highContrast: boolean;
  largerText: boolean;
  underlineLinks: boolean;
  focusRing: FocusRingPref;
  chatAnnounce: ChatAnnouncePref;
  approvalAnnounceAssertive: boolean;
  showSkipLinks: boolean;
};

export function defaultExtensionPreferences(): ExtensionPreferences {
  return {
    agentHostPath: "",
    openPanelOnStartup: false,
    autoFocusComposer: true,
    showFollowupHints: true,
    rememberPermissionMode: true,
    agentPickerEnabled: true,
    bypassPermissionsEnabled: false,
    compactionThresholdPercent: null,
    compactionThresholdTokens: 80_000,
    compactionTailTurns: 4,
    compactionPrune: true,
    compactionPruneRecencyTokens: 12_000,
    customInstructions: "",
    snippetsJson: "[]",
    reduceMotion: "system",
    highContrast: false,
    largerText: false,
    underlineLinks: false,
    focusRing: "default",
    chatAnnounce: "polite",
    approvalAnnounceAssertive: true,
    showSkipLinks: true,
  };
}

export function isExtensionSettingKey(value: string): value is ExtensionSettingKey {
  return (EXTENSION_SETTING_KEYS as readonly string[]).includes(value);
}

export type SnippetPref = {
  id: string;
  handle: string;
  body: string;
};

export function parseSnippetsJson(raw: string): SnippetPref[] {
  try {
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) {
      return [];
    }
    return parsed
      .filter((item): item is Record<string, unknown> => Boolean(item) && typeof item === "object")
      .map((item, index) => ({
        id: typeof item.id === "string" ? item.id : `snippet-${index}`,
        handle: typeof item.handle === "string" ? item.handle : "",
        body: typeof item.body === "string" ? item.body : "",
      }))
      .filter((item) => item.handle.trim().length > 0);
  } catch {
    return [];
  }
}

export function serializeSnippets(snippets: SnippetPref[]): string {
  return JSON.stringify(snippets);
}

export function coerceExtensionPreferences(
  raw: Record<string, unknown> | null | undefined,
): ExtensionPreferences {
  const base = defaultExtensionPreferences();
  if (!raw) {
    return base;
  }
  return {
    agentHostPath:
      typeof raw.agentHostPath === "string" ? raw.agentHostPath : base.agentHostPath,
    openPanelOnStartup: bool(raw.openPanelOnStartup, base.openPanelOnStartup),
    autoFocusComposer: bool(raw.autoFocusComposer, base.autoFocusComposer, true),
    showFollowupHints: bool(raw.showFollowupHints, base.showFollowupHints, true),
    rememberPermissionMode: bool(
      raw.rememberPermissionMode,
      base.rememberPermissionMode,
      true,
    ),
    agentPickerEnabled: bool(raw.agentPickerEnabled, base.agentPickerEnabled, true),
    bypassPermissionsEnabled: bool(
      raw.bypassPermissionsEnabled,
      base.bypassPermissionsEnabled,
    ),
    compactionThresholdPercent: nullableNumber(raw.compactionThresholdPercent),
    compactionThresholdTokens:
      int(raw.compactionThresholdTokens, base.compactionThresholdTokens) ??
      base.compactionThresholdTokens,
    compactionTailTurns:
      int(raw.compactionTailTurns, base.compactionTailTurns) ??
      base.compactionTailTurns,
    compactionPrune: bool(raw.compactionPrune, base.compactionPrune, true),
    compactionPruneRecencyTokens:
      int(raw.compactionPruneRecencyTokens, base.compactionPruneRecencyTokens) ??
      base.compactionPruneRecencyTokens,
    customInstructions:
      typeof raw.customInstructions === "string"
        ? raw.customInstructions
        : base.customInstructions,
    snippetsJson:
      typeof raw.snippetsJson === "string" ? raw.snippetsJson : base.snippetsJson,
    reduceMotion: oneOf(raw.reduceMotion, ["system", "always", "never"], base.reduceMotion),
    highContrast: bool(raw.highContrast, base.highContrast),
    largerText: bool(raw.largerText, base.largerText),
    underlineLinks: bool(raw.underlineLinks, base.underlineLinks),
    focusRing: oneOf(raw.focusRing, ["default", "strong"], base.focusRing),
    chatAnnounce: oneOf(
      raw.chatAnnounce,
      ["off", "polite", "assertive"],
      base.chatAnnounce,
    ),
    approvalAnnounceAssertive: bool(
      raw.approvalAnnounceAssertive,
      base.approvalAnnounceAssertive,
      true,
    ),
    showSkipLinks: bool(raw.showSkipLinks, base.showSkipLinks, true),
  };
}

function bool(
  value: unknown,
  fallback: boolean,
  trueDefaultWhenUndefined?: boolean,
): boolean {
  if (typeof value === "boolean") {
    return value;
  }
  if (value === undefined && trueDefaultWhenUndefined) {
    return true;
  }
  return fallback;
}

function int(value: unknown, fallback: number): number | null {
  if (typeof value === "number" && Number.isFinite(value)) {
    return Math.round(value);
  }
  if (typeof value === "string" && value.trim() !== "") {
    const n = Number(value);
    if (Number.isFinite(n)) {
      return Math.round(n);
    }
  }
  return fallback;
}

function nullableNumber(value: unknown): number | null {
  if (value === null || value === undefined || value === "") {
    return null;
  }
  if (typeof value === "number" && Number.isFinite(value)) {
    return Math.round(value);
  }
  if (typeof value === "string") {
    const n = Number(value);
    return Number.isFinite(n) ? Math.round(n) : null;
  }
  return null;
}

function oneOf<T extends string>(
  value: unknown,
  allowed: readonly T[],
  fallback: T,
): T {
  return typeof value === "string" && (allowed as readonly string[]).includes(value)
    ? (value as T)
    : fallback;
}

/** Extension setting value types accepted by WorkspaceAdapter.update. */
export function isValidSettingValue(
  key: ExtensionSettingKey,
  value: unknown,
): value is string | boolean | number | null {
  switch (key) {
    case "agentHostPath":
    case "customInstructions":
    case "snippetsJson":
    case "reduceMotion":
    case "focusRing":
    case "chatAnnounce":
      return typeof value === "string";
    case "compactionThresholdPercent":
      return (
        value === null ||
        (typeof value === "number" && Number.isFinite(value)) ||
        value === ""
      );
    case "compactionThresholdTokens":
    case "compactionTailTurns":
    case "compactionPruneRecencyTokens":
      return typeof value === "number" && Number.isFinite(value);
    default:
      return typeof value === "boolean";
  }
}
