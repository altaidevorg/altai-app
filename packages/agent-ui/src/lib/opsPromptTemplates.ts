/**
 * Pure Operations create-form prompt templates (A6.217).
 */

export type OpsPromptTemplate = {
  label: string;
  /** Primary instruction text for the create form. */
  prompt: string;
};

/** Work / Task Runs create form chip templates. */
export const TASK_PROMPT_TEMPLATES: readonly OpsPromptTemplate[] = [
  {
    label: "Fix a bug",
    prompt:
      "Investigate the reported bug, identify the root cause, implement the smallest safe fix, and run the relevant checks.",
  },
  {
    label: "Review changes",
    prompt:
      "Review the current working-tree changes for correctness, regressions, security risks, and missing tests. Make only clearly necessary fixes and report the findings.",
  },
  {
    label: "Add tests",
    prompt:
      "Inspect the relevant implementation, add focused tests for the important behavior and edge cases, then run the narrowest useful test command.",
  },
  {
    label: "Refactor safely",
    prompt:
      "Find the highest-value local refactor in the relevant area. Preserve behavior, keep the diff focused, and verify the result with appropriate checks.",
  },
] as const;

/** Automations create form chip templates (message field). */
export const AUTOMATION_PROMPT_TEMPLATES: readonly OpsPromptTemplate[] = [
  {
    label: "Code health",
    prompt:
      "Review the latest workspace changes for regressions, risky patterns, and missing tests. Return a concise, prioritized report with file references.",
  },
  {
    label: "Test failures",
    prompt:
      "Run the relevant test suite, investigate any failures, and return the root cause with the smallest safe fix or a clear recommended next step.",
  },
  {
    label: "Project brief",
    prompt:
      "Summarize meaningful workspace changes since the previous run. Highlight completed work, open risks, decisions, and the next three priorities.",
  },
] as const;

/** Map automation template shape used by older hosts (`message` key). */
export function automationTemplatesAsMessages(
  templates: readonly OpsPromptTemplate[] = AUTOMATION_PROMPT_TEMPLATES,
): ReadonlyArray<{ label: string; message: string }> {
  return templates.map((template) => ({
    label: template.label,
    message: template.prompt,
  }));
}
