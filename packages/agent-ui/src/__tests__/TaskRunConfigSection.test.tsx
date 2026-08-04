import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { TaskRunConfigSection } from "../components/TaskRunConfigSection.js";

describe("TaskRunConfigSection", () => {
  it("renders header and children", () => {
    const html = renderToStaticMarkup(
      createElement(
        TaskRunConfigSection,
        null,
        createElement("button", null, "Agent"),
        createElement("button", null, "Model"),
      ),
    );
    expect(html).toContain("Run configuration");
    expect(html).toContain("Agent");
    expect(html).toContain("Model");
  });
});
