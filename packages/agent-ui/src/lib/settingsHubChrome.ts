/**
 * Pure settings-hub section catalog + nav gating (A6.83).
 * Hosts map capabilities into flags; secrets stay host-side.
 */

export const SETTINGS_HUB_SECTION_DEFS = [
  {
    id: "general",
    label: "General",
    description:
      "Appearance, startup, and agent behavior. The host provides the editor shell; ALTAI owns agent prefs.",
  },
  {
    id: "shortcuts",
    label: "Shortcuts",
    description:
      "Keyboard shortcuts for ALTAI commands. Customize via the host keybindings editor when available.",
  },
  {
    id: "models",
    label: "Models",
    description:
      "Default model, failover model, and cloud provider credentials (secrets stay in the host).",
  },
  {
    id: "context",
    label: "Context",
    description:
      "Conversation compaction, prune budget, and workspace .isanagentignore.",
  },
  {
    id: "agents",
    label: "Agents",
    description:
      "Custom instructions and #snippets for the composer (Studio Agents equivalent).",
  },
  {
    id: "skills",
    label: "Skills",
    description: "Installed agent skills for this workspace.",
  },
  {
    id: "github",
    label: "GitHub",
    description:
      "GitHub account for git workflows. Use host GitHub auth or the CLI token flow.",
  },
  {
    id: "languages",
    label: "Languages",
    description:
      "Language servers are provided by the host / other extensions — not a second ALTAI LSP stack.",
  },
  {
    id: "mcp",
    label: "MCP",
    description: "Model Context Protocol servers from the agent host.",
  },
  {
    id: "hooks",
    label: "Hooks",
    description:
      "Inspect project WORKFLOW.md / lifecycle hooks for the active workspace root.",
  },
  {
    id: "accessibility",
    label: "Accessibility",
    description:
      "Motion, contrast, text size, focus, and announcements in the ALTAI panel.",
  },
  {
    id: "host",
    label: "Host",
    description:
      "Native agent host path, trust, project root, and recovery tools.",
  },
  {
    id: "about",
    label: "About",
    description: "Version, diagnostics, and recovery tools.",
  },
] as const;

export type SettingsHubSectionId =
  (typeof SETTINGS_HUB_SECTION_DEFS)[number]["id"];

export type SettingsHubCapabilityFlags = {
  canProvider: boolean;
  canModel: boolean;
  canPermission: boolean;
  canCompaction?: boolean;
  canMcp: boolean;
  canSkills: boolean;
};

export type SettingsHubNavItem = {
  id: SettingsHubSectionId;
  label: string;
  description: string;
  available: boolean;
};

export function listSettingsHubNav(
  caps: SettingsHubCapabilityFlags,
): SettingsHubNavItem[] {
  return SETTINGS_HUB_SECTION_DEFS.map((section) => {
    let available = true;
    switch (section.id) {
      case "models":
        available =
          caps.canProvider || caps.canModel || Boolean(caps.canCompaction);
        // Always openable: empty state explains host requirement for live data.
        available = true;
        break;
      case "skills":
        available = true;
        break;
      case "mcp":
        available = true;
        break;
      default:
        available = true;
    }
    return {
      id: section.id,
      label: section.label,
      description: section.description,
      available,
    };
  });
}

export function listSettingsHubSections(
  input: SettingsHubCapabilityFlags,
): SettingsHubSectionId[] {
  return listSettingsHubNav(input).map((item) => item.id);
}

export function normalizeSettingsHubSection(
  raw: string | undefined | null,
  available: readonly SettingsHubSectionId[],
): SettingsHubSectionId {
  // Legacy ids from earlier hubs
  if (raw === "provider" || raw === "model" || raw === "permission") {
    return available.includes("models") ? "models" : (available[0] ?? "general");
  }
  if (raw && available.includes(raw as SettingsHubSectionId)) {
    return raw as SettingsHubSectionId;
  }
  return available[0] ?? "general";
}
