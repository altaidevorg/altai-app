import { describe, expect, it } from "vitest";
import {
  buildTranscriptPartGroups,
  cmdSummaryForToolPart,
  formatGroupPreview,
  groupKindFor,
  pathBasename,
  toolNameOf,
  uniqueReadPaths,
  webSummaryForToolPart,
  type ToolLikePart,
} from "../lib/transcriptToolGroups.js";

describe("transcriptToolGroups", () => {
  it("normalizes static and dynamic tool names", () => {
    expect(toolNameOf({ type: "tool-read_file" })).toBe("read_file");
    expect(toolNameOf({ type: "dynamic-tool", toolName: "exec" })).toBe("exec");
    expect(toolNameOf({ type: "text" })).toBeNull();
  });

  it("does not group approval-requested tool parts", () => {
    expect(
      groupKindFor({
        type: "dynamic-tool",
        toolName: "read_file",
        state: "approval-requested",
      }),
    ).toBeNull();
    expect(
      groupKindFor({ type: "tool-read_file", state: "output-available" }),
    ).toBe("reads");
  });

  it("collapses consecutive same-kind tools (≥2)", () => {
    const parts: ToolLikePart[] = [
      { type: "tool-read_file", toolCallId: "a", input: { path: "a.ts" } },
      { type: "tool-read_file", toolCallId: "b", input: { path: "b.ts" } },
      { type: "text" },
      { type: "dynamic-tool", toolName: "exec", toolCallId: "c", input: { command: "ls" } },
    ];
    const groups = buildTranscriptPartGroups(parts);
    expect(groups).toHaveLength(3);
    expect(groups[0]).toMatchObject({ kind: "reads", parts: [parts[0], parts[1]] });
    expect(groups[1]).toMatchObject({ kind: "single", part: parts[2] });
    expect(groups[2]).toMatchObject({ kind: "single", part: parts[3] });
  });

  it("leaves a single collapsible tool as single", () => {
    const parts: ToolLikePart[] = [
      { type: "tool-read_file", toolCallId: "a", input: { path: "only.ts" } },
    ];
    const groups = buildTranscriptPartGroups(parts);
    expect(groups).toEqual([
      {
        kind: "single",
        part: parts[0],
        idx: 0,
        key: "a",
      },
    ]);
  });

  it("summarizes research and command tool inputs", () => {
    expect(
      webSummaryForToolPart({
        type: "dynamic-tool",
        toolName: "web_search",
        input: { query: "altai agent" },
      }),
    ).toBe('"altai agent"');
    expect(
      webSummaryForToolPart({
        type: "tool-web_fetch",
        input: { url: "https://example.com/docs" },
      }),
    ).toBe("example.com");
    expect(
      cmdSummaryForToolPart({
        type: "dynamic-tool",
        toolName: "exec",
        input: { command: "git status\nextra" },
      }),
    ).toBe("git status");
  });

  it("collects unique read paths and formats previews", () => {
    const parts: ToolLikePart[] = [
      { type: "tool-read_file", input: { path: "src/a.ts" } },
      { type: "tool-read_file", input: { path: "src/a.ts" } },
      { type: "tool-read_file", input: { path: "src/b.ts" } },
    ];
    const paths = uniqueReadPaths(parts);
    expect(paths).toEqual(["src/a.ts", "src/b.ts"]);
    expect(pathBasename(paths[0]!)).toBe("a.ts");
    expect(formatGroupPreview(["a", "b", "c", "d"], { max: 3 })).toBe(
      "a, b, c, +1 more",
    );
    expect(formatGroupPreview(["x", "y"], { separator: " · " })).toBe("x · y");
  });
});
