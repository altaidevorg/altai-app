import { describe, expect, it } from "vitest";
import { pruneOldToolOutputs } from "../lib/pruneToolOutputs.js";
import {
  CLEARED_OUTPUT,
  estimateTokens,
  isClearedOutput,
} from "../lib/tokenEstimate.js";

type Msg = {
  id: string;
  role: string;
  parts: unknown[];
};

function toolMsg(
  id: string,
  output: unknown,
  state: "output-available" | "output-error" = "output-available",
): Msg {
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
      },
    ],
  };
}

function textMsg(id: string, text: string): Msg {
  return { id, role: "user", parts: [{ type: "text", text }] };
}

function outputOf(msg: Msg): unknown {
  const part = msg.parts[0] as { output?: unknown };
  return part?.output;
}

describe("estimateTokens (re-export surface)", () => {
  it("returns ~4 chars per token rounded up", () => {
    expect(estimateTokens("abcd")).toBe(1);
  });
});

describe("pruneOldToolOutputs", () => {
  it("returns the input when there are no messages", () => {
    const empty: Msg[] = [];
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
      toolMsg("old", "x".repeat(40)),
      toolMsg("new", "y".repeat(40)),
    ];
    expect(pruneOldToolOutputs(msgs, 100)).toBe(msgs);
  });

  it("clears the oldest tool outputs when budget is exceeded", () => {
    const msgs = [
      toolMsg("a", "x".repeat(40)),
      toolMsg("b", "x".repeat(40)),
      toolMsg("c", "x".repeat(40)),
      toolMsg("d", "x".repeat(40)),
    ];
    const out = pruneOldToolOutputs(msgs, 25);
    expect(isClearedOutput(outputOf(out[0]!))).toBe(true);
    expect(isClearedOutput(outputOf(out[1]!))).toBe(true);
    expect(outputOf(out[2]!)).toBe("x".repeat(40));
    expect(outputOf(out[3]!)).toBe("x".repeat(40));
  });

  it("does not double-clear already-cleared outputs (idempotent)", () => {
    const msgs = [
      toolMsg("a", "x".repeat(100)),
      toolMsg("b", "y".repeat(100)),
    ];
    const first = pruneOldToolOutputs(msgs, 10);
    const second = pruneOldToolOutputs(first, 10);
    expect(second).toBe(first);
  });

  it("leaves errored tool outputs untouched", () => {
    const msgs = [
      toolMsg("err", "x".repeat(1000), "output-error"),
      toolMsg("ok", "y".repeat(1000)),
    ];
    const out = pruneOldToolOutputs(msgs, 1);
    const errPart = out[0]!.parts[0] as { state?: string; errorText?: string };
    expect(errPart.state).toBe("output-error");
    expect(errPart.errorText).toBe("boom");
    expect(isClearedOutput(outputOf(out[1]!))).toBe(true);
  });

  it("preserves non-tool parts verbatim", () => {
    const text = textMsg("txt", "important user text");
    const old = toolMsg("old", "x".repeat(1000));
    const msgs = [text, old];
    const out = pruneOldToolOutputs(msgs, 1);
    expect(out[0]).toBe(text);
    expect(isClearedOutput(outputOf(out[1]!))).toBe(true);
  });

  it("handles object outputs by counting their JSON size", () => {
    const obj = { content: "x".repeat(200) };
    const msgs = [toolMsg("a", obj)];
    const out = pruneOldToolOutputs(msgs, 1);
    expect(isClearedOutput(outputOf(out[0]!))).toBe(true);
    expect(isClearedOutput(CLEARED_OUTPUT)).toBe(true);
  });

  it("does not mutate the input array", () => {
    const msgs = [toolMsg("a", "x".repeat(1000))];
    const snapshot = JSON.parse(JSON.stringify(msgs));
    pruneOldToolOutputs(msgs, 1);
    expect(msgs).toEqual(snapshot);
  });
});
