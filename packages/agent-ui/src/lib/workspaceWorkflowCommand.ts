/**
 * Pure workspace workflow command parse (A6.157).
 * Host supplies icon / UI chrome; package owns Markdown + frontmatter shape.
 */

import {
  isValidSlashCommandName,
  workspaceSlashCommandStem,
} from "./workspaceSlashPath.js";

/** Parsed workspace slash workflow without host-only fields (e.g. icon). */
export type ParsedWorkspaceWorkflow = {
  name: string;
  invocation: string;
  label: string;
  description: string;
  aliases?: string[];
  category: "project";
  behavior: "workflow";
  source: "workspace";
  workflowPath: string;
  workflowInstructions: string;
};

/** Split a frontmatter aliases field into normalized command names. */
export function parseWorkflowAliases(value: string | undefined): string[] {
  if (!value) return [];
  return value
    .replace(/^\[|\]$/g, "")
    .split(",")
    .map((alias) => alias.trim().replace(/^\//, "").toLowerCase())
    .filter(Boolean);
}

const FRONTMATTER_RE = /^---\s*\r?\n([\s\S]*?)\r?\n---\s*(?:\r?\n|$)/;

/**
 * Parse a `.altai/commands/*.md` workflow into a host-ready command shape
 * (no icon). Returns null when path/name/body/aliases are invalid.
 */
export function parseWorkspaceWorkflowCommand(
  path: string,
  source: string,
): ParsedWorkspaceWorkflow | null {
  const fallbackName = workspaceSlashCommandStem(path);
  if (!fallbackName) return null;

  const frontmatter = source.match(FRONTMATTER_RE);
  const fields = new Map<string, string>();
  if (frontmatter) {
    for (const line of frontmatter[1]!.split(/\r?\n/)) {
      const entry = line.match(/^([a-zA-Z][\w-]*):\s*(.+?)\s*$/);
      if (entry) {
        fields.set(
          entry[1]!.toLowerCase(),
          entry[2]!.replace(/^['"]|['"]$/g, ""),
        );
      }
    }
  }

  const name = (fields.get("name") ?? fallbackName).toLowerCase();
  if (!isValidSlashCommandName(name)) return null;

  const body = (frontmatter ? source.slice(frontmatter[0]!.length) : source).trim();
  if (!body) return null;

  const heading = body.match(/^#\s+(.+)$/m)?.[1]?.trim();
  const description =
    fields.get("description") ?? heading ?? `Run workspace workflow from ${path}.`;
  const aliases = parseWorkflowAliases(fields.get("aliases"));
  if (aliases.some((alias) => !isValidSlashCommandName(alias))) return null;

  return {
    name,
    invocation: `/${name}`,
    label: fields.get("title") ?? heading ?? name,
    description,
    aliases: aliases.length ? aliases : undefined,
    category: "project",
    behavior: "workflow",
    source: "workspace",
    workflowPath: path,
    workflowInstructions: body,
  };
}
