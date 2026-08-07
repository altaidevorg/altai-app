import { describe, expect, it } from "vitest";
import {
  composePromptWithSnippets,
  expandSnippetTokens,
  findSnippets,
  insertSnippetHandle,
  mergeSnippetCatalogs,
  normalizeHandle,
  parseWorkspaceSnippetsJson,
  type ComposerSnippet,
} from "../lib/composerSnippets.js";

const CATALOG: readonly ComposerSnippet[] = [
  {
    id: "pr",
    handle: "pr",
    name: "PR review",
    description: "Review the current diff",
    content: "Review as a PR.",
  },
  {
    id: "explain",
    handle: "explain",
    name: "Explain",
    description: "Deep explain",
    content: "Explain deeply.",
  },
];

describe("normalizeHandle / findSnippets", () => {
  it("normalizes handles and filters catalog", () => {
    expect(normalizeHandle("Team Style")).toBe("team-style");
    expect(findSnippets(CATALOG, "pr").map((s) => s.handle)).toEqual(["pr"]);
  });
});

describe("expandSnippetTokens", () => {
  it("expands known tokens and strips handles", () => {
    const { body, blocks, matched } = expandSnippetTokens(
      "Please #pr carefully",
      CATALOG,
    );
    expect(matched.some((s) => s.handle === "pr")).toBe(true);
    expect(blocks[0]).toContain('<snippet name="pr">');
    expect(body).toBe("Please carefully");
  });

  it("leaves unknown tokens", () => {
    const { body, blocks } = expandSnippetTokens("use #nonesuch", CATALOG);
    expect(body).toContain("#nonesuch");
    expect(blocks).toHaveLength(0);
  });
});

describe("composePromptWithSnippets", () => {
  it("prepends blocks from tokens and picks", () => {
    const { prompt, matched } = composePromptWithSnippets(
      "hello",
      CATALOG,
      CATALOG.filter((s) => s.handle === "explain"),
    );
    expect(matched.some((s) => s.handle === "explain")).toBe(true);
    expect(prompt).toContain('<snippet name="explain">');
    expect(prompt).toContain("hello");
  });
});

describe("insertSnippetHandle / catalogs", () => {
  it("replaces an open #query token", () => {
    expect(insertSnippetHandle("hi #p", { start: 3, end: 5 }, "pr")).toBe(
      "hi #pr ",
    );
  });

  it("parses workspace json and merges catalogs", () => {
    const workspace = parseWorkspaceSnippetsJson(
      JSON.stringify([{ handle: "Team Style", content: "Follow style.md" }]),
    );
    expect(workspace[0]?.handle).toBe("team-style");
    const merged = mergeSnippetCatalogs(CATALOG, workspace);
    expect(merged.some((s) => s.handle === "team-style")).toBe(true);
  });
});
