import type { Agent } from "./agents";
import type { ModelInfo } from "../config";

/** Built-in chat agents all inspect the workspace or invoke tools while working. */
export function agentRequiresTools(agent: Agent | undefined): boolean {
  return !!agent;
}

/**
 * Models with an explicit capability record must support the active agent's
 * tools. Untagged local/custom models stay selectable because ALTAI cannot
 * reliably infer their server capabilities.
 */
export function supportsAgentModel(
  model: ModelInfo,
  agent: Agent | undefined,
): boolean {
  if (!agentRequiresTools(agent) || !model.tags) return true;
  return model.tags.includes("tools");
}

export function describeModelConstraint(agent: Agent | undefined): string | null {
  return agentRequiresTools(agent)
    ? `${agent?.name ?? "This agent"} needs a model that supports tools.`
    : null;
}

type AutoModelInput = {
  models: readonly ModelInfo[];
  agent: Agent | undefined;
  prompt?: string;
};

/**
 * Choose a concrete, compatible model for the next run. This is deliberately
 * deterministic and local: it never sends prompt content to a routing service.
 */
export function pickAutoModel({ models, agent, prompt = "" }: AutoModelInput): ModelInfo | null {
  const compatible = models.filter((model) => supportsAgentModel(model, agent));
  const candidates = compatible.length > 0 ? compatible : models;
  if (candidates.length === 0) return null;

  const normalized = prompt.toLowerCase();
  const complex =
    prompt.trim().length > 600 ||
    /\b(refactor|architecture|architect|security|migration|investigat|research|multi[- ]step|performance|debug)\b/.test(normalized) ||
    agent?.id === "builtin:architect" ||
    agent?.id === "builtin:security";

  return candidates
    .map((model) => {
      const { intelligence, speed, cost } = model.capabilities;
      const coding = model.tags?.includes("coding") ? 1 : 0;
      const tools = model.tags?.includes("tools") ? 1 : 0;
      const score = complex
        ? intelligence * 5 + speed + cost * 0.35 + coding * 1.5 + tools
        : speed * 4 + cost * 2 + intelligence + coding + tools;
      return { model, score };
    })
    .sort((a, b) => b.score - a.score || a.model.label.localeCompare(b.model.label))[0]
    ?.model ?? null;
}
