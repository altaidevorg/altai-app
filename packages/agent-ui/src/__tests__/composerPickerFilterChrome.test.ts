import { describe, expect, it } from "vitest";
import {
  filterSnippetsForPicker,
  filterWorkspacePathsForPicker,
} from "../lib/composerPickerFilterChrome.js";

describe("filterSnippetsForPicker", () => {
  const snippets = [
    { handle: "PR", name: "Pull Request", description: "Open a PR" },
    { handle: "fix", name: "Bugfix", description: "Fix defects" },
  ];

  it("returns all when query empty", () => {
    expect(filterSnippetsForPicker(snippets, "  ").map((s) => s.handle)).toEqual(
      ["PR", "fix"],
    );
  });

  it("matches handle case-sensitively and name/description loosely", () => {
    expect(filterSnippetsForPicker(snippets, "PR").map((s) => s.handle)).toEqual(
      ["PR"],
    );
    expect(filterSnippetsForPicker(snippets, "bug").map((s) => s.handle)).toEqual(
      ["fix"],
    );
  });
});

describe("filterWorkspacePathsForPicker", () => {
  const files = ["src/a.ts", "src/b.tsx", "docs/readme.md"];

  it("caps without query", () => {
    expect(filterWorkspacePathsForPicker(files, "", 2)).toEqual([
      "src/a.ts",
      "src/b.tsx",
    ]);
  });

  it("filters and caps", () => {
    expect(filterWorkspacePathsForPicker(files, "SRC", 10)).toEqual([
      "src/a.ts",
      "src/b.tsx",
    ]);
  });
});
