/**
 * Pure validation for Operations new-task form drafts (A6.101).
 */

export type TaskRunDraft = {
  title: string;
  prompt: string;
};

export type TaskRunDraftResult =
  | { ok: true; draft: TaskRunDraft }
  | { ok: false; error: string };

const MAX_TITLE = 120;
const MAX_PROMPT = 20_000;

/**
 * Trim and validate host-owned title/prompt before `work.createTaskRun`.
 */
export function validateTaskRunDraft(input: {
  title: string;
  prompt: string;
}): TaskRunDraftResult {
  const title = input.title.trim();
  const prompt = input.prompt.trim();
  if (!title) {
    return { ok: false, error: "Title is required." };
  }
  if (title.length > MAX_TITLE) {
    return { ok: false, error: `Title must be at most ${MAX_TITLE} characters.` };
  }
  if (!prompt) {
    return { ok: false, error: "Instruction prompt is required." };
  }
  if (prompt.length > MAX_PROMPT) {
    return {
      ok: false,
      error: `Prompt must be at most ${MAX_PROMPT} characters.`,
    };
  }
  return { ok: true, draft: { title, prompt } };
}

/** Toggle skill name selection for create-task chips (max 12). */
export function toggleTaskSkillSelection(
  selected: readonly string[],
  skillName: string,
): string[] {
  const name = skillName.trim();
  if (!name) {
    return [...selected];
  }
  if (selected.includes(name)) {
    return selected.filter((s) => s !== name);
  }
  return [...selected, name].slice(-12);
}

/**
 * Append selected skill names to the instruction so the host can honor them
 * without a dedicated skills field on createTaskRun.
 */
export function composeTaskPromptWithSkills(
  prompt: string,
  skillNames: readonly string[],
): string {
  const body = prompt.trim();
  const skills = skillNames.map((s) => s.trim()).filter(Boolean);
  if (skills.length === 0) {
    return body;
  }
  const block = [
    "<skills>",
    ...skills.map((name) => `  <skill name="${name}" />`),
    "</skills>",
  ].join("\n");
  return `${body}\n\n${block}`.trim();
}
