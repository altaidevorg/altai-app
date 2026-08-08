import { describe, expect, it } from "vitest";
import {
  composeTaskPromptWithSkills,
  toggleTaskSkillSelection,
  validateTaskRunDraft,
} from "../lib/taskRunDraft.js";

describe("validateTaskRunDraft", () => {
  it("accepts trimmed title and prompt", () => {
    expect(
      validateTaskRunDraft({
        title: "  Fix login  ",
        prompt: "  Investigate auth regression  ",
      }),
    ).toEqual({
      ok: true,
      draft: { title: "Fix login", prompt: "Investigate auth regression" },
    });
  });

  it("rejects empty fields", () => {
    expect(validateTaskRunDraft({ title: " ", prompt: "x" })).toMatchObject({
      ok: false,
    });
    expect(validateTaskRunDraft({ title: "t", prompt: "  " })).toMatchObject({
      ok: false,
    });
  });
});

describe("task skill selection", () => {
  it("toggles and caps at twelve", () => {
    expect(toggleTaskSkillSelection([], " dig ")).toEqual(["dig"]);
    expect(toggleTaskSkillSelection(["dig"], "dig")).toEqual([]);
    const many = Array.from({ length: 13 }, (_, i) => `s${i}`);
    let selected: string[] = [];
    for (const name of many) {
      selected = toggleTaskSkillSelection(selected, name);
    }
    expect(selected).toHaveLength(12);
  });
});

describe("composeTaskPromptWithSkills", () => {
  it("appends skill block", () => {
    const prompt = composeTaskPromptWithSkills("Do the thing", ["a", "b"]);
    expect(prompt).toContain("Do the thing");
    expect(prompt).toContain('<skill name="a"');
  });
});
