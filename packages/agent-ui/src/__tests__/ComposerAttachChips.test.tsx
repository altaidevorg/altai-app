import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import {
  ComposerAttachChips,
  fileExtensionLabel,
  selectionLineCount,
} from "../components/ComposerAttachChips.js";

describe("ComposerAttachChips helpers", () => {
  it("counts selection lines and labels extensions", () => {
    expect(selectionLineCount("a\nb\n")).toBe(2);
    expect(selectionLineCount("")).toBe(0);
    expect(fileExtensionLabel("App.tsx")).toBe("TSX");
    expect(fileExtensionLabel("Makefile")).toBe("FILE");
  });
});

describe("ComposerAttachChips", () => {
  it("returns null when empty", () => {
    const html = renderToStaticMarkup(
      createElement(ComposerAttachChips, {
        files: [],
        snippets: [],
        commands: [],
        onRemoveFile: () => {},
        onRemoveSnippet: () => {},
        onRemoveCommand: () => {},
      }),
    );
    expect(html).toBe("");
  });

  it("renders command, snippet, file chips and token estimate", () => {
    const html = renderToStaticMarkup(
      createElement(ComposerAttachChips, {
        commands: [
          {
            name: "review",
            label: "Review changes",
            icon: createElement("span", null, "icon"),
          },
        ],
        snippets: [{ id: "s1", handle: "boilerplate", description: "Boiler" }],
        files: [
          {
            id: "f1",
            name: "notes.md",
            kind: "text",
          },
          {
            id: "f2",
            name: "sel.ts",
            kind: "selection",
            source: "editor",
            text: "one\ntwo\nthree",
          },
        ],
        onRemoveFile: () => {},
        onRemoveSnippet: () => {},
        onRemoveCommand: () => {},
        contextTokenEstimate: 1500,
      }),
    );
    expect(html).toContain("#review");
    expect(html).toContain("boilerplate");
    expect(html).toContain("MD");
    expect(html).toContain("notes.md");
    expect(html).toContain("· 3L");
    expect(html).toContain("~1.5k tokens");
    expect(html).toContain("Remove command");
  });
});
