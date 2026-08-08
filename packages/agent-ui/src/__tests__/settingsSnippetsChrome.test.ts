import { describe, expect, it } from "vitest";
import {
  mergeSnippetCatalogFromPrefs,
  prefsToComposerSnippets,
} from "../lib/settingsSnippetsChrome.js";
import type { ComposerSnippet } from "../lib/composerSnippets.js";

const DEFAULTS: ComposerSnippet[] = [
  {
    id: "builtin-pr",
    handle: "pr",
    name: "PR review",
    description: "Review the current diff",
    content: "review the pr",
  },
];

describe("settingsSnippetsChrome", () => {
  it("maps preference rows into composer snippets", () => {
    const mapped = prefsToComposerSnippets([
      { id: "user-1", handle: "Team Style", body: "use team style" },
    ]);
    expect(mapped).toEqual([
      {
        id: "user-1",
        handle: "team-style",
        name: "#team-style",
        description: "Custom snippet from Settings → Agents",
        content: "use team style",
      },
    ]);
  });

  it("lets user prefs win on matching handle", () => {
    const merged = mergeSnippetCatalogFromPrefs(DEFAULTS, [
      {
        id: "user-1",
        handle: "pr",
        body: "custom body from settings",
      },
    ]);
    const hit = merged.find((s) => s.handle === "pr");
    expect(hit?.content).toBe("custom body from settings");
    expect(hit?.id).toBe("user-1");
  });
});
