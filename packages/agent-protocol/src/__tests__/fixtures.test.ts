import { describe, expect, it } from "vitest";
import { readdir, readFile } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { encodeFrame, FrameDecoder, FrameError } from "../frame.js";
import { JsonRpcErrorCode, MAX_JSON_DEPTH, RunSequenceTracker, validateMessage } from "../schema.js";

const fixturesDirectory = fileURLToPath(
  new URL("../../../../shared/agent-protocol/v1/fixtures/", import.meta.url),
);

describe("shared protocol v1 fixtures", () => {
  it("accepts every Rust/TypeScript golden fixture", async () => {
    const names = (await readdir(fixturesDirectory)).filter((name) => name.endsWith(".json"));
    for (const name of names) {
      const fixture = JSON.parse(await readFile(join(fixturesDirectory, name), "utf8")) as { message: unknown; valid?: boolean };
      expect(validateMessage(fixture.message).ok, name).toBe(fixture.valid ?? true);
    }
  });

  it("rejects missing ids, invalid run identity, invalid sequence, and excessive nesting", () => {
    expect(validateMessage({ jsonrpc: "2.0", method: "initialize", id: null })).toMatchObject({
      ok: false,
      code: JsonRpcErrorCode.InvalidRequest,
    });
    expect(validateMessage({ jsonrpc: "2.0", method: "run/event", params: { chat_id: "", run_id: "r", seq: 0, event: {} } })).toMatchObject({
      ok: false,
      code: JsonRpcErrorCode.InvalidRunIdentity,
    });
    expect(validateMessage({ jsonrpc: "2.0", id: "init", method: "initialize", params: { protocol_min: 2, protocol_max: 2 } })).toMatchObject({
      ok: false,
      code: JsonRpcErrorCode.UnsupportedProtocol,
    });
    let deep: unknown = null;
    for (let index = 0; index < MAX_JSON_DEPTH; index += 1) deep = [deep];
    expect(validateMessage({ jsonrpc: "2.0", method: "workspace/status", params: deep })).toMatchObject({ ok: false, code: JsonRpcErrorCode.InvalidRequest });
    const tracker = new RunSequenceTracker();
    expect(tracker.observe("chat", "run", 1)).toBeUndefined();
    expect(tracker.observe("chat", "run", 1)).toMatchObject({ ok: false, code: JsonRpcErrorCode.SequenceViolation });
    expect(validateMessage({ jsonrpc: "2.0", id: "future", method: "future/method" })).toMatchObject({ ok: false, code: JsonRpcErrorCode.MethodNotFound });
    expect(validateMessage({ jsonrpc: "2.0", id: "event", method: "run/event", params: {} })).toMatchObject({ ok: false, code: JsonRpcErrorCode.InvalidRequest });
    expect(validateMessage({ jsonrpc: "2.0", method: "run/start" })).toMatchObject({ ok: false, code: JsonRpcErrorCode.InvalidRequest });
    expect(validateMessage({ jsonrpc: "2.0", id: "response", error: { code: -32001 } })).toMatchObject({ ok: false, code: JsonRpcErrorCode.InvalidRequest });
  });
});

describe("LSP-style framing", () => {
  const encoder = new TextEncoder();
  const decoder = new TextDecoder();

  it("accepts partial headers/bodies and multiple frames", () => {
    const first = encodeFrame(encoder.encode('{"one":1}'));
    const second = encodeFrame(encoder.encode('{"two":2}'));
    const frames = new FrameDecoder();
    expect(frames.push(first.slice(0, 7))).toEqual([]);
    expect(frames.push(concat(first.slice(7), second)).map((frame) => decoder.decode(frame))).toEqual(['{"one":1}', '{"two":2}']);
  });

  it("rejects malformed and oversized frames", () => {
    expect(() => new FrameDecoder().push(encoder.encode("Length: 2\r\n\r\n{}"))).toThrow(FrameError);
    expect(() => new FrameDecoder({ maxHeaderBytes: 32, maxFrameBytes: 3 }).push(encoder.encode("Content-Length: 4\r\n\r\n1234"))).toThrow(FrameError);
  });
});

function concat(first: Uint8Array, second: Uint8Array): Uint8Array {
  const combined = new Uint8Array(first.length + second.length);
  combined.set(first);
  combined.set(second, first.length);
  return combined;
}
