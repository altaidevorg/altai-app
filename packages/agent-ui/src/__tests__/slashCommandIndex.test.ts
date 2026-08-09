import { describe, expect, it } from "vitest";
import {
  filterSlashCommands,
  resolveSlashCommandInIndex,
} from "../lib/slashCommandIndex.js";

const index = [
  {
    name: "docs",
    label: "Update documentation",
    description: "Inspect the change",
    aliases: ["document"],
    category: "project",
    source: "builtin",
  },
  {
    name: "mcp",
    label: "MCP settings",
    description: "Open MCP",
    category: "settings",
    source: "builtin",
  },
] as const;

describe("filterSlashCommands", () => {
  it("returns full index for empty query", () => {
    expect(filterSlashCommands(index, "")).toBe(index);
    expect(filterSlashCommands(index, "   ")).toBe(index);
  });

  it("matches name label description alias category", () => {
    expect(filterSlashCommands(index, "DOC").map((c) => c.name)).toEqual(["docs"]);
    expect(filterSlashCommands(index, "document").map((c) => c.name)).toEqual(["docs"]);
    expect(filterSlashCommands(index, "settings").map((c) => c.name)).toEqual(["mcp"]);
  });
});

describe("resolveSlashCommandInIndex", () => {
  it("resolves by name and alias", () => {
    expect(resolveSlashCommandInIndex(index, "docs")?.name).toBe("docs");
    expect(resolveSlashCommandInIndex(index, "Document")?.name).toBe("docs");
    expect(resolveSlashCommandInIndex(index, "nope")).toBeUndefined();
  });
});
