import { describe, expect, it } from "vitest";
import {
  activityKindForTool,
  parseMcpToolName,
} from "../lib/mcpToolName.js";

describe("parseMcpToolName", () => {
  it("parses canonical and legacy shapes", () => {
    expect(parseMcpToolName("mcp__github__list_issues")).toEqual({
      server: "Github",
      tool: "List Issues",
    });
    expect(parseMcpToolName("mcp_github_list_issues")).toEqual({
      server: "Github",
      tool: "List Issues",
    });
    expect(parseMcpToolName("read_file")).toBe(null);
  });
});

describe("activityKindForTool", () => {
  it("classifies mcp, research, and generic tools", () => {
    expect(activityKindForTool("mcp__x__y")).toBe("mcp");
    expect(activityKindForTool("web_search")).toBe("research");
    expect(activityKindForTool("edit_file")).toBe("tool");
  });
});
