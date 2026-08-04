import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { TaskSkillChips } from "../components/TaskSkillChips.js";

describe("TaskSkillChips", () => {
  it("renders nothing when there are no skills", () => {
    const html = renderToStaticMarkup(
      createElement(TaskSkillChips, {
        skills: [],
        selected: [],
        onToggle: () => {},
      }),
    );
    expect(html).toBe("");
  });

  it("renders selected and unselected chips", () => {
    const html = renderToStaticMarkup(
      createElement(TaskSkillChips, {
        skills: [
          { name: "review", description: "Code review playbook" },
          { name: "test" },
        ],
        selected: ["review"],
        onToggle: () => {},
      }),
    );
    expect(html).toContain("Skills");
    expect(html).toContain("review");
    expect(html).toContain("test");
    expect(html).toContain('aria-pressed="true"');
    expect(html).toContain('aria-pressed="false"');
    expect(html).toContain("Code review playbook");
  });
});
