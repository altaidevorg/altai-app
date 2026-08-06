/**
 * Grow a textarea with content without exceeding `maxPx`.
 * Wave 4 / A6.12 — DOM-side helper, host-owned styling still applies.
 */
export function autoresizeTextarea(
  el: HTMLTextAreaElement | null,
  options?: { maxPx?: number },
): void {
  if (!el) return;
  const maxPx = options?.maxPx ?? 176;
  // Clear first so a stale inline height can't keep the box tall after shrink.
  el.style.height = "";
  if (el.value.length === 0) return;
  el.style.height = `${Math.min(el.scrollHeight, maxPx)}px`;
}
