/**
 * Built-in ALTAI agent profiles for the composer switcher (A6.88).
 * Matches Desktop identity ids / labels; host owns selection state.
 */

export type ComposerAgentIconId =
  | "coder"
  | "architect"
  | "reviewer"
  | "security"
  | "designer"
  | "spark";

export type ComposerAgentProfile = {
  id: string;
  name: string;
  description: string;
  icon: ComposerAgentIconId;
  /** Short system persona prepended to the user prompt for this host. */
  promptPrefix: string;
};

export const DEFAULT_COMPOSER_AGENT_ID = "builtin:coder";

/** Profiles shown in the composer agent switcher (built-ins only). */
export const COMPOSER_AGENT_PROFILES: readonly ComposerAgentProfile[] = [
  {
    id: "builtin:coder",
    name: "Coder",
    description: "General-purpose coding assistant. Writes, edits, and runs.",
    icon: "coder",
    promptPrefix:
      "You are Coder — an expert software engineer pair-programming in this workspace. Prefer the smallest correct change and match existing patterns.",
  },
  {
    id: "builtin:architect",
    name: "Architect",
    description: "Design and tradeoffs. Plans before code.",
    icon: "architect",
    promptPrefix:
      "You are Architect — a senior software architect. Restate the problem, surface viable approaches with tradeoffs, and recommend one with risks.",
  },
  {
    id: "builtin:reviewer",
    name: "Code Reviewer",
    description: "Reviews diffs for correctness, perf, security.",
    icon: "reviewer",
    promptPrefix:
      "You are Code Reviewer — a meticulous reviewer. Focus on logic, edge cases, races, security, and data integrity. Skip pure formatting nits.",
  },
  {
    id: "builtin:security",
    name: "Security",
    description: "Threat-models changes and flags vulns.",
    icon: "security",
    promptPrefix:
      "You are Security — an application-security engineer. Threat-model boundaries and report severity + concrete fixes.",
  },
  {
    id: "builtin:designer",
    name: "Designer",
    description: "UI/UX critique and refinement.",
    icon: "designer",
    promptPrefix:
      "You are Designer — a senior product designer. Critique hierarchy, spacing, density, and affordance with concrete UI changes.",
  },
  {
    id: "builtin:spark",
    name: "Adaptive",
    description: "Flexible generalist for exploratory work.",
    icon: "spark",
    promptPrefix:
      "You are Adaptive — a flexible generalist. Clarify goals, explore options briefly, then execute with clear checkpoints.",
  },
];

export function resolveComposerAgent(
  id: string | null | undefined,
): ComposerAgentProfile {
  const found = COMPOSER_AGENT_PROFILES.find((agent) => agent.id === id);
  return (
    found ??
    COMPOSER_AGENT_PROFILES.find(
      (agent) => agent.id === DEFAULT_COMPOSER_AGENT_ID,
    ) ??
    COMPOSER_AGENT_PROFILES[0]!
  );
}

export function canMountAgentPicker(options: {
  agentPickerEnabled: boolean;
}): boolean {
  return options.agentPickerEnabled;
}

/**
 * Prefix the run prompt with the selected agent persona once.
 * Avoids double-prefixing if the same header is already present.
 */
export function applyAgentPromptPrefix(
  prompt: string,
  agent: ComposerAgentProfile,
): string {
  const text = prompt.trimStart();
  const header = `[ALTAI agent: ${agent.name}]`;
  if (text.startsWith(header)) {
    return prompt;
  }
  const body = prompt.trim().length > 0 ? prompt : "";
  if (!body) {
    return `${header}\n${agent.promptPrefix}`;
  }
  return `${header}\n${agent.promptPrefix}\n\n${body}`;
}
