import { describe, expect, it } from "vitest";
import { mapBackendMessageToTranscript } from "../lib/backendMessageMap.js";

describe("mapBackendMessageToTranscript", () => {
  it("maps text user message", () => {
    const out = mapBackendMessageToTranscript(
      { role: "user", content: "  hi  " },
      0,
    );
    expect(out).toEqual({
      id: "backend-0",
      role: "user",
      parts: [{ type: "text", text: "hi" }],
    });
  });

  it("folds reasoning, tool calls, and remaps tool role", () => {
    const out = mapBackendMessageToTranscript(
      {
        role: "tool",
        content: "result",
        reasoning_content: "think",
        tool_calls: [
          {
            id: "tc1",
            function: { name: "read_file", arguments: '{"path":"a"}' },
          },
          {
            id: "tc2",
            function: { name: "bad", arguments: "not-json" },
          },
        ],
      },
      3,
    );
    expect(out.id).toBe("backend-3");
    expect(out.role).toBe("assistant");
    expect(out.parts[0]).toEqual({ type: "text", text: "think" });
    expect(out.parts[1]).toMatchObject({
      type: "dynamic-tool",
      toolName: "read_file",
      toolCallId: "tc1",
      input: { path: "a" },
      state: "input-available",
    });
    expect(out.parts[2]).toMatchObject({
      toolCallId: "tc2",
      input: { raw: "not-json" },
    });
    expect(out.parts[3]).toEqual({ type: "text", text: "result" });
  });
});
