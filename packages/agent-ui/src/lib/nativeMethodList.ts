/**
 * Pure parse for host-advertised native RPC method lists (A6.141).
 * Used after initialize / host.getCapabilities bridges.
 */

/**
 * Accept only an array of strings (method names). Returns null when the
 * payload is missing or malformed so hosts can choose pending vs locked.
 */
export function parseNativeMethodList(
  value: unknown,
): readonly string[] | null {
  if (!Array.isArray(value)) {
    return null;
  }
  if (!value.every((item) => typeof item === "string")) {
    return null;
  }
  return value as readonly string[];
}

/**
 * Whether a method is available given an advertised list.
 * `null`/`undefined` list = pending (treat as available until handshake resolves).
 * Empty list = locked (nothing available).
 */
export function nativeMethodAvailable(
  methods: readonly string[] | null | undefined,
  method: string,
): boolean {
  if (methods === null || methods === undefined) {
    return true;
  }
  return methods.includes(method);
}
