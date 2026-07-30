import { FrameDecoder, encodeFrame, validateMessage, type JsonRpcId, type ProtocolMessage } from "@altai/agent-protocol";
import { randomUUID } from "node:crypto";

export type Writable = { write(data: Uint8Array, callback?: (error?: Error | null) => void): boolean; end(): void };
export type Readable = { on(event: string, listener: (...args: unknown[]) => void): unknown };
export type ProcessLike = {
  readonly stdin: Writable;
  readonly stdout: Readable;
  readonly stderr: Readable;
  on(event: string, listener: (...args: unknown[]) => void): unknown;
  kill(signal?: NodeJS.Signals | number): boolean;
};

export type ProtocolNotification = Extract<ProtocolMessage, { method: string }>;

type PendingRequest = { resolve(value: unknown): void; reject(error: Error): void; timer: ReturnType<typeof setTimeout> };

/** A tiny, typed-at-the-boundary JSON-RPC client over the shared framed protocol. */
export class RpcClient {
  private readonly decoder = new FrameDecoder();
  private readonly pending = new Map<JsonRpcId, PendingRequest>();
  private readonly notifications = new Set<(message: ProtocolNotification) => void>();
  private closed = false;

  constructor(private readonly process: ProcessLike, private readonly onLog: (line: string) => void) {
    process.stdout.on("data", (chunk) => this.onData(chunk as Uint8Array));
    process.stderr.on("data", (chunk) => this.onLog(redact(String(chunk))));
    process.on("error", (error) => this.failAll(error as Error));
    process.on("exit", (code, signal) => this.failAll(new Error(`ALTAI host exited (${String(code ?? signal ?? "unknown")})`)));
  }

  request(method: string, params?: unknown, timeoutMs = 10_000): Promise<unknown> {
    if (this.closed) return Promise.reject(new Error("ALTAI host is closed"));
    const id = randomUUID();
    const message: ProtocolMessage = { jsonrpc: "2.0", id, method, ...(params === undefined ? {} : { params }) };
    const validation = validateMessage(message);
    if (!validation.ok) return Promise.reject(new Error(`Invalid protocol request: ${validation.reason}`));
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(id);
        reject(new Error(`ALTAI host request timed out: ${method}`));
      }, timeoutMs);
      this.pending.set(id, { resolve, reject, timer });
      this.write(message);
    });
  }

  notify(method: string, params?: unknown): void {
    if (this.closed) return;
    const message: ProtocolMessage = { jsonrpc: "2.0", method, ...(params === undefined ? {} : { params }) };
    const validation = validateMessage(message);
    if (!validation.ok) throw new Error(`Invalid protocol notification: ${validation.reason}`);
    this.write(message);
  }

  /** Subscribe to validated server notifications such as `run/event`. */
  onNotification(listener: (message: ProtocolNotification) => void): () => void {
    this.notifications.add(listener);
    return () => this.notifications.delete(listener);
  }

  async shutdown(timeoutMs = 1_500): Promise<void> {
    if (this.closed) return;
    try {
      await this.request("shutdown", undefined, timeoutMs);
    } catch (error) {
      this.onLog(`Graceful host shutdown failed: ${error instanceof Error ? error.message : "unknown error"}`);
    } finally {
      this.close();
    }
  }

  close(): void {
    if (this.closed) return;
    this.closed = true;
    this.process.stdin.end();
    this.process.kill("SIGTERM");
    this.failAll(new Error("ALTAI host closed"));
  }

  private onData(chunk: Uint8Array): void {
    try {
      for (const body of this.decoder.push(chunk)) {
        const message = JSON.parse(new TextDecoder().decode(body)) as unknown;
        const validation = validateMessage(message);
        if (!validation.ok) {
          this.onLog(`Ignoring invalid protocol message: ${validation.reason}`);
          continue;
        }
        if ("id" in validation.message && !("method" in validation.message)) {
          const pending = this.pending.get(validation.message.id);
          if (!pending) continue;
          clearTimeout(pending.timer);
          this.pending.delete(validation.message.id);
          if ("error" in validation.message && validation.message.error) {
            pending.reject(new Error(protocolErrorMessage(validation.message.error)));
          } else {
            pending.resolve(validation.message.result);
          }
        } else if ("method" in validation.message && !("id" in validation.message)) {
          for (const listener of this.notifications) {
            try {
              listener(validation.message);
            } catch (error) {
              this.onLog(`ALTAI notification listener failed: ${error instanceof Error ? error.message : "unknown error"}`);
            }
          }
        }
      }
    } catch (error) {
      this.failAll(error instanceof Error ? error : new Error("Invalid ALTAI host frame"));
    }
  }

  private write(message: ProtocolMessage): void {
    const payload = new TextEncoder().encode(JSON.stringify(message));
    this.process.stdin.write(encodeFrame(payload), (error) => {
      if (error) this.failAll(error);
    });
  }

  private failAll(error: Error): void {
    if (this.closed && this.pending.size === 0) return;
    this.closed = true;
    for (const pending of this.pending.values()) {
      clearTimeout(pending.timer);
      pending.reject(error);
    }
    this.pending.clear();
  }
}

function protocolErrorMessage(error: unknown): string {
  if (typeof error === "object" && error !== null && "message" in error && typeof error.message === "string") return error.message;
  return "ALTAI host returned an error";
}

function redact(line: string): string {
  return line.replace(/(authorization|api[_-]?key|token)\s*[:=]\s*\S+/gi, "$1=<redacted>");
}
