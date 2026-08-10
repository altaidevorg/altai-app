import { describe, expect, it } from "vitest";
import {
  AUTOMATION_PROMPT_TEMPLATES,
  TASK_PROMPT_TEMPLATES,
  automationTemplatesAsMessages,
} from "../lib/opsPromptTemplates.js";

describe("ops prompt templates", () => {
  it("exposes non-empty labels and prompts", () => {
    expect(TASK_PROMPT_TEMPLATES.length).toBeGreaterThan(0);
    expect(AUTOMATION_PROMPT_TEMPLATES.length).toBeGreaterThan(0);
    for (const t of [...TASK_PROMPT_TEMPLATES, ...AUTOMATION_PROMPT_TEMPLATES]) {
      expect(t.label.trim().length).toBeGreaterThan(0);
      expect(t.prompt.trim().length).toBeGreaterThan(10);
    }
  });
  it("maps automation templates to message field", () => {
    const msgs = automationTemplatesAsMessages();
    expect(msgs[0]).toEqual({
      label: AUTOMATION_PROMPT_TEMPLATES[0].label,
      message: AUTOMATION_PROMPT_TEMPLATES[0].prompt,
    });
  });
});
