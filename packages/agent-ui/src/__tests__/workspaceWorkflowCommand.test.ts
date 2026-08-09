import { describe, expect, it } from "vitest";
import {
  parseWorkflowAliases,
  parseWorkspaceWorkflowCommand,
} from "../lib/workspaceWorkflowCommand.js";

describe("parseWorkflowAliases", () => {
  it("parses bare and bracketed lists", () => {
    expect(parseWorkflowAliases(undefined)).toEqual([]);
    expect(parseWorkflowAliases("foo, /Bar,")).toEqual(["foo", "bar"]);
    expect(parseWorkflowAliases("[a, b]")).toEqual(["a", "b"]);
  });
});

describe("parseWorkspaceWorkflowCommand", () => {
  it("returns null for invalid path", () => {
    expect(parseWorkspaceWorkflowCommand("src/x.md", "body")).toBeNull();
  });

  it("requires non-empty body", () => {
    expect(
      parseWorkspaceWorkflowCommand(".altai/commands/do.md", "   "),
    ).toBeNull();
  });

  it("parses frontmatter + body", () => {
    const src = `---
name: release
title: Release notes
description: Ship notes
aliases: notes, /rel
---
# Heading ignored when description set

Ship it.
`;
    const parsed = parseWorkspaceWorkflowCommand(
      ".altai/commands/release-notes.md",
      src,
    );
    expect(parsed).toMatchObject({
      name: "release",
      invocation: "/release",
      label: "Release notes",
      description: "Ship notes",
      aliases: ["notes", "rel"],
      category: "project",
      behavior: "workflow",
      source: "workspace",
      workflowPath: ".altai/commands/release-notes.md",
    });
    expect(parsed?.workflowInstructions).toContain("Ship it.");
  });

  it("uses path stem and first heading when frontmatter sparse", () => {
    const src = `# My workflow\n\nDo things.`;
    const parsed = parseWorkspaceWorkflowCommand(
      ".altai/commands/my-work.md",
      src,
    );
    expect(parsed).toMatchObject({
      name: "my-work",
      label: "My workflow",
      description: "My workflow",
    });
    expect(parsed?.aliases).toBeUndefined();
  });

  it("rejects invalid aliases", () => {
    expect(
      parseWorkspaceWorkflowCommand(
        ".altai/commands/ok.md",
        `---\naliases: bad space\n---\nbody`,
      ),
    ).toBeNull();
  });
});
