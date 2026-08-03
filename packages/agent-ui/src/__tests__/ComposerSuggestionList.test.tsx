import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import {
  ComposerSuggestionList,
  type ComposerSuggestionItem,
} from "../components/ComposerSuggestionList.js";

describe("ComposerSuggestionList", () => {
  it("shows empty slash-command copy", () => {
    const html = renderToStaticMarkup(
      createElement(ComposerSuggestionList, {
        items: [],
        activeIndex: 0,
        onPick: () => {},
        onHover: () => {},
        commandPrefix: "/",
      }),
    );
    expect(html).toContain("No slash commands match.");
  });

  it("renders command and snippet rows with active highlight", () => {
    const items: ComposerSuggestionItem[] = [
      {
        kind: "command",
        name: "init",
        label: "Initialize workspace",
        description: "Draft ALTAI.md",
        category: "workspace",
        aliases: ["bootstrap"],
        icon: createElement("span", { "data-icon": "cmd" }, "*"),
      },
      {
        kind: "snippet",
        id: "sn-1",
        handle: "review",
        name: "Code review",
        description: "Review checklist",
      },
    ];
    const html = renderToStaticMarkup(
      createElement(ComposerSuggestionList, {
        items,
        activeIndex: 1,
        onPick: () => {},
        onHover: () => {},
        commandPrefix: "/",
      }),
    );
    expect(html).toContain("Slash commands");
    expect(html).toContain("/init");
    expect(html).toContain("Initialize workspace");
    expect(html).toContain("aliases: /bootstrap");
    expect(html).toContain("Snippets");
    expect(html).toContain("#review");
    expect(html).toContain("bg-foreground/[0.065]");
  });

  it("keeps slash aliases even when the picker prefix is #", () => {
    const html = renderToStaticMarkup(
      createElement(ComposerSuggestionList, {
        items: [
          {
            kind: "command",
            name: "retry",
            label: "Retry",
            description: "Rerun",
            category: "session",
            aliases: ["regenerate"],
            icon: null,
          },
        ],
        activeIndex: 0,
        onPick: () => {},
        onHover: () => {},
        commandPrefix: "#",
      }),
    );
    expect(html).toContain("#retry");
    expect(html).toContain("aliases: /regenerate");
  });
});
