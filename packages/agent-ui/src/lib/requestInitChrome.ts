/**
 * Pure request URL/method normalize for proxy fetch paths (A6.177).
 */

/** Coerce fetch `input` into a URL string. */
export function requestUrlToString(input: unknown): string {
  if (typeof input === "string") return input;
  if (input instanceof URL) return input.toString();
  if (
    typeof input === "object" &&
    input !== null &&
    "url" in input &&
    typeof (input as { url: unknown }).url === "string"
  ) {
    return (input as { url: string }).url;
  }
  return String(input);
}

/** Uppercase HTTP method from RequestInit (default GET). */
export function requestMethodFromInit(
  init: { method?: string } | null | undefined,
): string {
  return (init?.method ?? "GET").toUpperCase();
}
