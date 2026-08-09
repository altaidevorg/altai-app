import { native } from "./native";
import {
  PROJECT_INSTRUCTIONS_FILE,
  MAX_PROJECT_INSTRUCTIONS_CHARS,
  projectInstructionsPath as projectInstructionsPathShared,
  combineAgentInstructions as combineAgentInstructionsShared,
  clampProjectInstructions,
} from "@altai/agent-ui";

export { PROJECT_INSTRUCTIONS_FILE };

export function projectInstructionsPath(workspacePath: string): string {
  return projectInstructionsPathShared(workspacePath);
}

/** Read the workspace contract without making an absent file an agent error. */
export async function readProjectInstructions(
  workspacePath: string | undefined,
): Promise<string | undefined> {
  if (!workspacePath) return undefined;
  try {
    const result = await native.readFile(projectInstructionsPath(workspacePath));
    if (result.kind !== "text") return undefined;
    return clampProjectInstructions(
      result.content,
      MAX_PROJECT_INSTRUCTIONS_CHARS,
    );
  } catch {
    return undefined;
  }
}

/**
 * IsanAgent receives one persona string at runtime creation. Put project
 * rules ahead of the selected persona so both survive per-session isolation.
 */
export function combineAgentInstructions(
  agentInstructions: string | undefined,
  projectInstructions: string | undefined,
): string | undefined {
  return combineAgentInstructionsShared(agentInstructions, projectInstructions);
}
