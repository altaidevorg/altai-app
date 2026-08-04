import { CodeIcon } from "@hugeicons/core-free-icons";
import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { AgentOptionRow } from "../components/AgentOptionRow.js";

describe("AgentOptionRow", () => {
  it("renders name, description, and selected checkmark", () => {
    const html = renderToStaticMarkup(
      createElement(AgentOptionRow, {
        name: "Coder",
        description: "Implements features",
        icon: CodeIcon,
        selected: true,
      }),
    );
    expect(html).toContain("Coder");
    expect(html).toContain("Implements features");
    expect(html).toContain("text-foreground");
  });

  it("omits description and keeps muted icon when requested", () => {
    const html = renderToStaticMarkup(
      createElement(AgentOptionRow, {
        name: "Custom bot",
        icon: CodeIcon,
        selected: true,
        iconAlwaysMuted: true,
      }),
    );
    expect(html).toContain("Custom bot");
    expect(html).not.toContain("line-clamp-1");
    expect(html).toContain("text-muted-foreground");
  });
});
