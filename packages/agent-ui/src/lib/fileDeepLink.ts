/**
 * Pure helpers for Extension Host → Webview "Ask about active file" deep-links (A6.120).
 * No React / vscode APIs.
 */

export type OpenChatWithFilePayload = {
  /** Unique key so remounted/idempotent opens re-apply. */
  key: number;
  uri: string;
  path: string;
  name: string;
};

function basenamePath(path: string): string {
  const parts = path.replace(/\\/g, "/").split("/");
  return parts[parts.length - 1] || path;
}

/**
 * Runtime-validate a `chat.attachFile` event payload.
 * Returns null when the payload is malformed.
 */
export function parseOpenChatWithFilePayload(
  value: unknown,
): OpenChatWithFilePayload | null {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    return null;
  }
  const record = value as Record<string, unknown>;
  if (typeof record.key !== "number" || !Number.isFinite(record.key)) {
    return null;
  }
  if (typeof record.uri !== "string" || record.uri.trim().length === 0) {
    return null;
  }
  if (typeof record.path !== "string" || record.path.trim().length === 0) {
    return null;
  }
  const name =
    typeof record.name === "string" && record.name.trim().length > 0
      ? record.name.trim()
      : basenamePath(record.path);
  return {
    key: record.key,
    uri: record.uri.trim(),
    path: record.path.trim(),
    name,
  };
}

export function buildOpenChatWithFilePayload(input: {
  uri: string;
  path: string;
  name?: string;
  key?: number;
}): OpenChatWithFilePayload | null {
  const uri = input.uri.trim();
  const path = input.path.trim();
  if (!uri || !path) {
    return null;
  }
  const name =
    input.name?.trim() && input.name.trim().length > 0
      ? input.name.trim()
      : basenamePath(path);
  return {
    key: input.key ?? Date.now(),
    uri,
    path,
    name,
  };
}
