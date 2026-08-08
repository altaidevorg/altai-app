/**
 * Pure helpers for Extension Host → Webview Settings surface deep-links (A6.123).
 */

export type OpenSettingsPayload = {
  key: number;
  section?: string;
};

export function parseOpenSettingsPayload(
  value: unknown,
): OpenSettingsPayload | null {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    return null;
  }
  const record = value as Record<string, unknown>;
  if (typeof record.key !== "number" || !Number.isFinite(record.key)) {
    return null;
  }
  const section =
    typeof record.section === "string" &&
    /^[a-z][a-z0-9-]*$/.test(record.section.trim())
      ? record.section.trim()
      : undefined;
  return {
    key: record.key,
    ...(section ? { section } : {}),
  };
}

export function buildOpenSettingsPayload(input?: {
  key?: number;
  section?: string;
}): OpenSettingsPayload {
  const section =
    typeof input?.section === "string" &&
    /^[a-z][a-z0-9-]*$/.test(input.section.trim())
      ? input.section.trim()
      : undefined;
  return {
    key: input?.key ?? Date.now(),
    ...(section ? { section } : {}),
  };
}
