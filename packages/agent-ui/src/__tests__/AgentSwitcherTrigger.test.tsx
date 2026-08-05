import { CodeIcon } from "@hugeicons/core-free-icons";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { AgentSwitcherTrigger } from "../components/AgentSwitcherTrigger.js";

describe("AgentSwitcherTrigger", () => {
  it("renders the default picker trigger", () => {
    const html = renderToStaticMarkup(
      createElement(AgentSwitcherTrigger, {
        name: "Coder",
        icon: CodeIcon,
      }),
    );

    expect(html).toContain("altai-agent-switcher-trigger");
    expect(html).toContain("Coder");
    expect(html).toContain('aria-label="Switch agent — current: Coder"');
    expect(html).toContain('title="Agent: Coder"');
  });

  it("reuses composer chrome for the toolbar variant", () => {
    const html = renderToStaticMarkup(
      createElement(AgentSwitcherTrigger, {
        name: "Reviewer",
        icon: CodeIcon,
        variant: "toolbar",
      }),
    );

    expect(html).toContain("altai-ai-composer-config-trigger");
    expect(html).toContain("Reviewer");
    expect(html).toContain("max-w-[9rem]");
  });

  it("keeps the toolbar icon variant visually compact", () => {
    const html = renderToStaticMarkup(
      createElement(AgentSwitcherTrigger, {
        name: "Architect",
        icon: CodeIcon,
        variant: "toolbar-icon",
      }),
    );

    expect(html).toContain("size-7");
    expect(html).toContain('aria-label="Switch agent — current: Architect"');
    expect(html).not.toContain(">Architect</span>");
  });
});
