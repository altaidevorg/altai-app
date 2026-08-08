import { describe, expect, it } from "vitest";
import {
  buildDisplayTranscriptBlocks,
  groupCountLabel,
  toolGroupKindFor,
} from "../lib/displayTranscriptBlocks.js";
import { groupKindFor, groupKindFromToolName } from "../lib/transcriptToolGroups.js";

describe("shared tool name catalogs", () => {
  it("classifies shell/bash aliases as cmd", () => {
    expect(groupKindFromToolName("shell")).toBe("cmd");
    expect(groupKindFromToolName("bash")).toBe("cmd");
    expect(groupKindFromToolName("view_file")).toBe("reads");
    expect(
      groupKindFor({ type: "dynamic-tool", toolName: "shell", state: "done" }),
    ).toBe("cmd");
  });
});

describe("toolGroupKindFor", () => {
  it("classifies known tools", () => {
    expect(
      toolGroupKindFor({
        id: "1",
        role: "tool",
        toolName: "read_file",
        content: "",
      }),
    ).toBe("reads");
    expect(
      toolGroupKindFor({
        id: "2",
        role: "tool",
        toolName: "exec",
        content: "",
      }),
    ).toBe("cmd");
    expect(
      toolGroupKindFor({
        id: "3",
        role: "user",
        content: "hi",
      }),
    ).toBeNull();
  });
});

describe("buildDisplayTranscriptBlocks", () => {
  it("groups consecutive reads", () => {
    const blocks = buildDisplayTranscriptBlocks([
      {
        id: "u",
        role: "user",
        content: "x",
      },
      {
        id: "t1",
        role: "tool",
        toolName: "read_file",
        content: "a",
        filePath: "a.ts",
      },
      {
        id: "t2",
        role: "tool",
        toolName: "read_file",
        content: "b",
        filePath: "b.ts",
      },
      {
        id: "a",
        role: "assistant",
        content: "done",
      },
    ]);
    expect(blocks).toHaveLength(3);
    expect(blocks[0]?.kind).toBe("message");
    expect(blocks[1]?.kind).toBe("tool-group");
    if (blocks[1]?.kind === "tool-group") {
      expect(blocks[1].messages).toHaveLength(2);
      expect(blocks[1].label).toBe("Read");
      expect(blocks[1].countLabel).toBe(groupCountLabel("reads", 2));
    }
    expect(blocks[2]?.kind).toBe("message");
  });

  it("keeps single tools ungrouped", () => {
    const blocks = buildDisplayTranscriptBlocks([
      {
        id: "t1",
        role: "tool",
        toolName: "shell",
        content: "ls",
      },
    ]);
    expect(blocks).toEqual([
      {
        kind: "message",
        message: {
          id: "t1",
          role: "tool",
          toolName: "shell",
          content: "ls",
        },
      },
    ]);
  });
});
