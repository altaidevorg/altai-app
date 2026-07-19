import { describe, expect, it } from "vitest";
import type { UIMessage } from "ai";
import {
  CLEARED_OUTPUT,
  estimateTokens,
  isClearedOutput,
  pruneOldToolOutputs,
} from "./compaction";

/** Build a UIMessage with a single dynamic-tool part carrying `output`. */
function toolMsg(
  id: string,
  output: unknown,
  state: "output-available" | "output-error" = "output-available",
): UIMessage {
  return {
    id,
    role: "assistant",
    parts: [
      {
        type: "dynamic-tool",
        toolName: "read_file",
        toolCallId: id,
        state,
        input: { path: "x" },
        ...(state === "output-error" ? { errorText: "boom" } : { output }),
      } as UIMessage["parts"][number],
    ],
  };
}

function textMsg(id: string, text: string): UIMessage {
  return { id, role: "user", parts: [{ type: "text", text }] };
}

function outputOf(msg: UIMessage): unknown {
  const part = msg.parts[0] as { output?: unknown };
  return part?.output;
}

describe("estimateTokens", () => {
  it("returns ~4 chars per token rounded up", () => {
    expect(estimateTokens("")).toBe(0);
    expect(estimateTokens("abcd")).toBe(1);
    expect(estimateTokens("abcde")).toBe(2);
    expect(estimateTokens("abcdefgh")).toBe(2);
  });
});

describe("isClearedOutput", () => {
  it("recognizes the cleared marker", () => {
    expect(isClearedOutput(CLEARED_OUTPUT)).toBe(true);
    expect(isClearedOutput({ cleared: true })).toBe(true);
  });
  it("rejects other shapes", () => {
    expect(isClearedOutput(null)).toBe(false);
    expect(isClearedOutput("hello")).toBe(false);
    expect(isClearedOutput({ cleared: false })).toBe(false);
    expect(isClearedOutput({ other: true })).toBe(false);
  });
});

describe("pruneOldToolOutputs", () => {
  it("returns the input when there are no messages", () => {
    const empty: UIMessage[] = [];
    expect(pruneOldToolOutputs(empty, 100)).toBe(empty);
  });

  it("returns the input when budget is non-positive", () => {
    const msgs = [toolMsg("a", "x".repeat(1000))];
    expect(pruneOldToolOutputs(msgs, 0)).toBe(msgs);
    expect(pruneOldToolOutputs(msgs, Number.NaN)).toBe(msgs);
    expect(pruneOldToolOutputs(msgs, -1)).toBe(msgs);
  });

  it("returns the input when there are no tool outputs", () => {
    const msgs = [textMsg("a", "hello"), textMsg("b", "world")];
    expect(pruneOldToolOutputs(msgs, 100)).toBe(msgs);
  });

  it("returns the input when every tool output fits the budget", () => {
    const msgs = [
      toolMsg("old", "x".repeat(40)), // 10 tokens
      toolMsg("new", "y".repeat(40)), // 10 tokens
    ];
    expect(pruneOldToolOutputs(msgs, 100)).toBe(msgs);
  });

  it("clears the oldest tool outputs when budget is exceeded", () => {
    // 4 tool outputs, 10 tokens each = 40 total. Budget = 25 keeps only
    // the newest ~2; the older ones get the cleared marker.
    const msgs = [
      toolMsg("a", "x".repeat(40)), // oldest
      toolMsg("b", "x".repeat(40)),
      toolMsg("c", "x".repeat(40)),
      toolMsg("d", "x".repeat(40)), // newest
    ];
    const out = pruneOldToolOutputs(msgs, 25);
    // Newest two kept, oldest two cleared.
    expect(isClearedOutput(outputOf(out[0]))).toBe(true);
    expect(isClearedOutput(outputOf(out[1]))).toBe(true);
    expect(outputOf(out[2])).toBe("x".repeat(40));
    expect(outputOf(out[3])).toBe("x".repeat(40));
  });

  it("does not double-clear already-cleared outputs (idempotent)", () => {
    const msgs = [
      toolMsg("a", "x".repeat(100)),
      toolMsg("b", "y".repeat(100)),
    ];
    const first = pruneOldToolOutputs(msgs, 10);
    // Re-run on the pruned result; the cleared outputs consume no budget so
    // the surviving newest output stays intact.
    const second = pruneOldToolOutputs(first, 10);
    expect(second).toBe(first);
  });

  it("leaves errored tool outputs untouched", () => {
    const msgs = [
      toolMsg("err", "x".repeat(1000), "output-error"),
      toolMsg("ok", "y".repeat(1000)),
    ];
    const out = pruneOldToolOutputs(msgs, 1);
    // Error part keeps its original shape (no `cleared` marker).
    const errPart = out[0].parts[0] as { state?: string; errorText?: string };
    expect(errPart.state).toBe("output-error");
    expect(errPart.errorText).toBe("boom");
    // The completed part is cleared.
    expect(isClearedOutput(outputOf(out[1]))).toBe(true);
  });

  it("preserves non-tool parts verbatim", () => {
    const text = textMsg("txt", "important user text");
    const old = toolMsg("old", "x".repeat(1000));
    const msgs = [text, old];
    const out = pruneOldToolOutputs(msgs, 1);
    expect(out[0]).toBe(text); // same reference — untouched
    expect(isClearedOutput(outputOf(out[1]))).toBe(true);
  });

  it("handles object outputs by counting their JSON size", () => {
    const obj = { content: "x".repeat(200) };
    const msgs = [toolMsg("a", obj)];
    // Small budget → cleared.
    const out = pruneOldToolOutputs(msgs, 1);
    expect(isClearedOutput(outputOf(out[0]))).toBe(true);
  });

  it("does not mutate the input array", () => {
    const msgs = [toolMsg("a", "x".repeat(1000))];
    const snapshot = JSON.parse(JSON.stringify(msgs)) as UIMessage[];
    pruneOldToolOutputs(msgs, 1);
    expect(msgs).toEqual(snapshot);
  });
});
