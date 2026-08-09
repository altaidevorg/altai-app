/**
 * Pure MCP tool name parse + activity kind (A6.142).
 * Matches the contract used by the Rust MCP host (`mcp__server__tool`).
 */

export type McpToolInfo = { server: string; tool: string };

function humanize(value: string): string {
  return value
    .replace(/[-_]+/g, " ")
    .replace(/\b\w/g, (letter) => letter.toUpperCase());
}

/**
 * Parse an agent-facing MCP tool name into its `{ server, tool }` parts.
 *
 * Canonical: `mcp__<server>__<tool>`
 * Legacy: `mcp_<server>_<tool>` (best-effort for older transcripts)
 */
export function parseMcpToolName(name: string): McpToolInfo | null {
  if (name.startsWith("mcp__")) {
    const parts = name.split("__");
    if (parts.length !== 3 || !parts[1] || !parts[2]) return null;
    return { server: humanize(parts[1]), tool: humanize(parts[2]) };
  }
  if (name.startsWith("mcp_")) {
    const [, server, ...toolParts] = name.split("_");
    if (!server || toolParts.length === 0) return null;
    return { server: humanize(server), tool: humanize(toolParts.join("_")) };
  }
  return null;
}

export const RESEARCH_TOOL_NAMES = new Set([
  "web_search",
  "web_fetch",
  "arxiv_search",
  "arxiv_fetch",
  "hf_hub_file_fetch",
]);

export type ToolActivityKind = "research" | "mcp" | "tool";

/** Classify a tool call for inspector / activity chrome. */
export function activityKindForTool(name: string): ToolActivityKind {
  if (parseMcpToolName(name)) return "mcp";
  return RESEARCH_TOOL_NAMES.has(name) ? "research" : "tool";
}
