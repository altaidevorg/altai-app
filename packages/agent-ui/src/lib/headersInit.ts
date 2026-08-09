/**
 * Pure HeadersInit → record normalize (A6.169).
 * Avoids assuming a global Headers type when running under bare Node.
 */

export type FlatHeaders = Record<string, string>;

/**
 * Convert fetch-style headers init into a plain string record.
 * Accepts Headers-like (forEach), array tuples, or plain object.
 */
export function headersInitToRecord(init: unknown): FlatHeaders | undefined {
  if (init == null) return undefined;
  const out: FlatHeaders = {};

  if (typeof (init as { forEach?: unknown }).forEach === "function") {
    (init as { forEach: (cb: (value: string, key: string) => void) => void }).forEach(
      (value, key) => {
        out[key] = value;
      },
    );
    return out;
  }

  if (Array.isArray(init)) {
    for (const entry of init) {
      if (!Array.isArray(entry) || entry.length < 2) continue;
      out[String(entry[0])] = String(entry[1]);
    }
    return out;
  }

  if (typeof init === "object") {
    for (const [k, v] of Object.entries(init as Record<string, unknown>)) {
      out[k] = String(v);
    }
    return out;
  }

  return undefined;
}
