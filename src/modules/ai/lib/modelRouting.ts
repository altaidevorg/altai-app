import type { Agent } from "./agents";
import type { ModelInfo } from "../config";
import {
  agentRequiresTools as agentRequiresToolsShared,
  supportsAgentModel as supportsAgentModelShared,
  describeModelConstraint as describeModelConstraintShared,
  pickAutoModel as pickAutoModelShared,
} from "@altai/agent-ui";

/** Built-in chat agents all inspect the workspace or invoke tools while working. */
export function agentRequiresTools(agent: Agent | undefined): boolean {
  return agentRequiresToolsShared(agent);
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
  return supportsAgentModelShared(model, agent);
}

export function describeModelConstraint(agent: Agent | undefined): string | null {
  return describeModelConstraintShared(agent);
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
export function pickAutoModel({
  models,
  agent,
  prompt = "",
}: AutoModelInput): ModelInfo | null {
  const picked = pickAutoModelShared({ models, agent, prompt });
  if (!picked) return null;
  return models.find((m) => m.id === picked.id) ?? null;
}
