/**
 * Pure slash command prompt templates (A6.200).
 * Focus tail uses appendSlashCommandFocus.
 */

import { appendSlashCommandFocus } from "./slashCommandFocus.js";

/** /init workspace ALTAI.md bootstrap prompt. */
export const INIT_WORKSPACE_PROMPT = `Scan this workspace and produce ALTAI.md at the workspace root with:

- One-paragraph project description.
- Build / test / dev commands.
- Architecture overview (subsystems, data flow, key dirs).
- Conventions worth knowing (naming, patterns, gotchas).
- Paths to entry points.

Use grep/glob/list_directory/read_file to explore. Cap ALTAI.md under 200 lines. Use write_file to create it (will go through normal approval).`;

const DEFAULT_SLASH_PROMPT =
  "Handle the requested task carefully and verify the result.";

/** Base prompt body by slash command name (without user focus tail). */
export const SLASH_COMMAND_PROMPTS: Readonly<Record<string, string>> = {
  init: INIT_WORKSPACE_PROMPT,
  index:
    "Inspect this workspace without changing files. Produce a compact codebase map: entry points, major modules, data flow, build/test commands, conventions, and high-risk areas. Cite concrete paths for each conclusion.",
  search:
    "Search the workspace for the requested concept. Report the most relevant paths and lines, explain how they connect, and do not make changes unless explicitly asked.",
  "git-status":
    "Inspect the Git repository state. Summarize branch/upstream, changed and untracked files, staged versus unstaged work, and the safest next step. Do not modify Git state.",
  diff: "Inspect the current working-tree diff. Summarize intent, affected areas, likely regressions, and missing verification. Do not apply changes.",
  explain:
    "Explain the requested code or behaviour accurately. Read the relevant workspace files first, cite paths, and do not change files.",
  fix: "Investigate the reported issue first. Identify the root cause, make the smallest focused fix, then run the most relevant verification and report evidence.",
  refactor:
    "Inspect the requested scope and existing conventions. Propose a focused refactor, preserve behaviour, make changes only after understanding dependencies, and verify the result.",
  todo: "Break this task into an ordered, concrete checklist using the todo tool. Include discovery, implementation, verification, and any approval boundary.",
  test: "Discover the project’s relevant test command from its configuration and documentation. Run the smallest relevant test scope first, diagnose failures, and report exact results.",
  lint: "Discover the project lint command, run it for the relevant scope, fix clear issues when appropriate, and report the final command result.",
  build:
    "Discover the production build command, run it, diagnose failures if any, and report the exact verification result.",
  review:
    "Review the requested change scope for correctness, regressions, maintainability, and missing tests. Read the diff and surrounding code; do not modify files unless explicitly asked.",
  security:
    "Perform a focused security review of the requested scope. Look for auth, authorization, injection, data exposure, dependency, and unsafe execution issues. Report only evidence-backed findings with paths and severity.",
  perf: "Review the requested scope for measurable performance risks. Inspect hot paths, rendering, I/O, network, and algorithmic complexity; propose changes with expected impact and verification.",
  docs: "Inspect the requested feature or change and update the documentation that users or maintainers need. Keep claims tied to the actual implementation and verify links/commands where possible.",
  workflow:
    "Inspect existing project automation and WORKFLOW.md. Propose or update a reusable workflow with clear trigger, steps, validation, approval boundaries, and rollback notes.",
  research:
    "Research the requested topic using primary, current sources where possible. Separate facts from inference, cite sources, and translate findings into concrete project implications.",
};

/** Resolve a slash command name + optional tail into a ready prompt. */
export function promptForSlashCommand(name: string, tail: string): string {
  return appendSlashCommandFocus(
    SLASH_COMMAND_PROMPTS[name] ?? DEFAULT_SLASH_PROMPT,
    tail,
  );
}
