/**
 * Filter Studio settings nav by free-text search (A6.84).
 * Hosts pass SettingsHubNavItem rows from settingsHubChrome.
 */

import type {
  SettingsHubNavItem,
  SettingsHubSectionId,
} from "./settingsHubChrome.js";

const SECTION_KEYWORDS: Record<SettingsHubSectionId, string[]> = {
  general: ["startup", "theme", "panel", "bypass", "agent host", "path", "focus"],
  shortcuts: ["keyboard", "keybinding", "hotkey", "shortcut"],
  models: [
    "model",
    "provider",
    "failover",
    "permission",
    "api",
    "key",
    "openai",
    "anthropic",
    "gemini",
  ],
  context: ["compact", "ignore", "isanagent", "prune", "token", "context"],
  agents: ["snippet", "instructions", "persona", "#"],
  skills: ["skill", "github", "install"],
  github: ["git", "account", "token", "auth"],
  languages: ["lsp", "language", "typescript", "python"],
  mcp: ["mcp", "server", "tool"],
  hooks: ["workflow", "lifecycle", "hook"],
  accessibility: [
    "a11y",
    "contrast",
    "motion",
    "screen reader",
    "aria",
    "focus",
  ],
  host: ["restart", "trust", "workspace", "root", "host"],
  about: ["version", "diagnostic", "license", "about"],
};

export function filterSettingsNav(
  nav: readonly SettingsHubNavItem[],
  query: string,
): SettingsHubNavItem[] {
  const q = query.trim().toLowerCase();
  if (!q) {
    return [...nav];
  }
  return nav.filter((item) => {
    const hay = [
      item.id,
      item.label,
      item.description,
      ...(SECTION_KEYWORDS[item.id] ?? []),
    ]
      .join(" ")
      .toLowerCase();
    return (
      hay.includes(q) || q.split(/\s+/).every((token) => hay.includes(token))
    );
  });
}

export function settingsSectionSearchScore(
  item: SettingsHubNavItem,
  query: string,
): number {
  const q = query.trim().toLowerCase();
  if (!q) {
    return 0;
  }
  let score = 0;
  if (item.label.toLowerCase().startsWith(q)) score += 3;
  if (item.label.toLowerCase().includes(q)) score += 2;
  if (item.id.includes(q)) score += 2;
  if (item.description.toLowerCase().includes(q)) score += 1;
  return score;
}
