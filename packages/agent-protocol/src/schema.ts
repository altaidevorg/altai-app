export const PROTOCOL_VERSION = 1 as const;
export const MAX_JSON_DEPTH = 64;

export const JsonRpcErrorCode = {
  ParseError: -32700,
  InvalidRequest: -32600,
  MethodNotFound: -32601,
  InvalidParams: -32602,
  InternalError: -32603,
  UnsupportedProtocol: -32001,
  InvalidRunIdentity: -32002,
  SequenceViolation: -32003,
  CapabilityUnavailable: -32004,
} as const;

const REQUEST_METHODS = new Set([
  "initialize",
  "workspace/status",
  "config/get",
  "config/update",
  "models/list",
  "providers/status",
  "providers/connect",
  "providers/clear",
  "mcp/servers/list",
  "mcp/servers/configure",
  "mcp/servers/enable",
  "mcp/servers/restart",
  "work/tasks/list",
  "work/tasks/create",
  "work/tasks/cancel",
  "work/tasks/retry",
  "work/tasks/remove",
  "work/automations/list",
  "work/automations/create",
  "work/automations/update",
  "work/automations/trigger",
  "work/automations/pause",
  "work/automations/delete",
  "agents/list",
  "sessions/list",
  "sessions/get",
  "sessions/create",
  "sessions/messages",
  "sessions/truncate",
  "sessions/rename",
  "sessions/archive",
  "sessions/delete",
  "inbox/list",
  "inbox/mark-seen",
  "inbox/resolve",
  "run/start",
  "run/steer",
  "run/cancel",
  "run/retry",
  "run/replay",
  "clarification/respond",
  "context/compact",
  "checkpoints/list",
  "checkpoints/restore",
  "review/proposals/list",
  "review/proposals/upsert",
  "review/proposals/apply",
  "review/proposals/deny",
  "shutdown",
]);
const NOTIFICATION_METHODS = new Set(["run/event", "workspace/changed", "host/log", "host/status"]);

export type JsonRpcId = string | number;
export type ProtocolMessage =
  | { jsonrpc: "2.0"; id: JsonRpcId; method: string; params?: unknown }
  | { jsonrpc: "2.0"; method: string; params?: unknown }
  | { jsonrpc: "2.0"; id: JsonRpcId; result?: unknown; error?: unknown };

export type ValidationResult =
  | { ok: true; message: ProtocolMessage }
  | { ok: false; code: number; reason: string };

export function validateMessage(value: unknown): ValidationResult {
  if (!isObject(value) || value.jsonrpc !== "2.0") {
    return invalid("jsonrpc_must_be_2_0");
  }
  if (jsonDepth(value) > MAX_JSON_DEPTH) return invalid("json_nesting_limit");

  if (typeof value.method === "string") {
    if (!value.method.trim()) return invalid("method_must_be_non_empty");
    if (!REQUEST_METHODS.has(value.method) && !NOTIFICATION_METHODS.has(value.method)) return failure(JsonRpcErrorCode.MethodNotFound, "method_not_found");
    if ("id" in value && !validId(value.id)) return invalid("request_id_invalid");
    if ("id" in value && !REQUEST_METHODS.has(value.method)) return invalid("notification_method_cannot_have_id");
    if (!("id" in value) && !NOTIFICATION_METHODS.has(value.method)) return invalid("request_method_requires_id");
    const initializeError = validateInitialize(value.method, value.params);
    if (initializeError) return initializeError;
    const runError = validateRunEvent(value.method, value.params);
    if (runError) return runError;
    return { ok: true, message: value as ProtocolMessage };
  }

  if (!validId(value.id)) return invalid("response_id_invalid");
  const hasResult = "result" in value;
  const hasError = "error" in value;
  if (hasResult === hasError) return invalid("response_requires_one_outcome");
  if (hasError && !validResponseError(value.error)) return invalid("response_error_shape_invalid");
  return { ok: true, message: value as ProtocolMessage };
}

function validResponseError(value: unknown): boolean {
  return isObject(value) && typeof value.code === "number" && typeof value.message === "string" && value.message.trim().length > 0;
}

export class RunSequenceTracker {
  private readonly lastByRun = new Map<string, number>();

  observe(chatId: string, runId: string, seq: number): ValidationResult | undefined {
    const key = `${chatId}\u0000${runId}`;
    const last = this.lastByRun.get(key);
    if (last !== undefined && seq <= last) {
      return failure(JsonRpcErrorCode.SequenceViolation, "run_sequence_not_monotonic");
    }
    this.lastByRun.set(key, seq);
    return undefined;
  }
}

function validateInitialize(method: string, params: unknown): ValidationResult | undefined {
  if (method !== "initialize") return undefined;
  if (!isObject(params) || typeof params.protocol_min !== "number" || typeof params.protocol_max !== "number" || !Number.isSafeInteger(params.protocol_min) || !Number.isSafeInteger(params.protocol_max) || params.protocol_min > params.protocol_max) {
    return failure(JsonRpcErrorCode.InvalidParams, "initialize_version_range_invalid");
  }
  if (params.protocol_min > PROTOCOL_VERSION || params.protocol_max < PROTOCOL_VERSION) {
    return failure(JsonRpcErrorCode.UnsupportedProtocol, "unsupported_protocol");
  }
  return undefined;
}

function validateRunEvent(method: string, params: unknown): ValidationResult | undefined {
  if (method !== "run/event") return undefined;
  if (!isObject(params)) return failure(JsonRpcErrorCode.InvalidRunIdentity, "run_event_params_invalid");
  if (typeof params.chat_id !== "string" || !params.chat_id.trim() || typeof params.run_id !== "string" || !params.run_id.trim()) {
    return failure(JsonRpcErrorCode.InvalidRunIdentity, "run_identity_invalid");
  }
  if (typeof params.seq !== "number" || !Number.isSafeInteger(params.seq) || params.seq <= 0) {
    return failure(JsonRpcErrorCode.SequenceViolation, "run_sequence_invalid");
  }
  if (!isObject(params.event)) return failure(JsonRpcErrorCode.InvalidParams, "run_event_payload_invalid");
  return undefined;
}

function validId(value: unknown): value is JsonRpcId {
  return (typeof value === "string" && value.trim().length > 0) || typeof value === "number";
}

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function jsonDepth(value: unknown): number {
  if (Array.isArray(value)) return 1 + Math.max(0, ...value.map(jsonDepth));
  if (isObject(value)) return 1 + Math.max(0, ...Object.values(value).map(jsonDepth));
  return 1;
}

function invalid(reason: string): ValidationResult {
  return failure(JsonRpcErrorCode.InvalidRequest, reason);
}

function failure(code: number, reason: string): ValidationResult {
  return { ok: false, code, reason };
}
