import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { PromptEditorSection } from "../components/PromptEditorSection.js";

describe("PromptEditorSection", () => {
  it("renders header, textarea, and templates", () => {
    const html = renderToStaticMarkup(
      createElement(PromptEditorSection, {
        title: "Describe the outcome",
        description: "Give the agent a concrete result.",
        value: "Fix auth",
        onChange: () => {},
        placeholder: "Example prompt",
        templates: [{ label: "Fix a bug", value: "fix it" }],
        textareaId: "background-task-prompt",
      }),
    );
    expect(html).toContain("Describe the outcome");
    expect(html).toContain("Example prompt");
    expect(html).toContain("Fix a bug");
    expect(html).toContain('id="background-task-prompt"');
    expect(html).toContain("Fix auth");
  });
});
