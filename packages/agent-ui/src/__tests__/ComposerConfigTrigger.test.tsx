import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { ComposerConfigTrigger } from "../components/ComposerConfigTrigger.js";

describe("ComposerConfigTrigger", () => {
  it("renders label, icon slot, and shared class contract", () => {
    const html = renderToStaticMarkup(
      createElement(ComposerConfigTrigger, {
        icon: createElement("span", { "data-icon": "agent" }, "i"),
        label: "Coder",
        "aria-label": "Switch agent — current: Coder",
        title: "Agent: Coder",
      }),
    );
    expect(html).toContain("altai-ai-composer-config-trigger");
    expect(html).toContain("altai-ai-composer-config-trigger-label");
    expect(html).toContain("Coder");
    expect(html).toContain('data-icon="agent"');
    expect(html).toContain('aria-label="Switch agent — current: Coder"');
    expect(html).toContain('type="button"');
  });

  it("merges caller className", () => {
    const html = renderToStaticMarkup(
      createElement(ComposerConfigTrigger, {
        icon: null,
        label: "Auto",
        className: "max-w-[9rem] text-warning",
      }),
    );
    expect(html).toContain("max-w-[9rem]");
    expect(html).toContain("text-warning");
  });
});
