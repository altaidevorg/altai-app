/**
 * Pure keyboard navigation for shell surface tabs (A6.73).
 * Host-neutral surface order; no vscode/Tauri imports.
 */

export const ALTAI_SURFACES = ["chat", "operations", "settings"] as const;

export type AltaiSurfaceId = (typeof ALTAI_SURFACES)[number];

export function nextAltaiSurface(
  current: AltaiSurfaceId,
  key: "ArrowLeft" | "ArrowRight" | "Home" | "End",
  order: readonly AltaiSurfaceId[] = ALTAI_SURFACES,
): AltaiSurfaceId {
  if (order.length === 0) {
    return current;
  }
  const idx = order.indexOf(current);
  const at = idx < 0 ? 0 : idx;
  if (key === "Home") {
    return order[0]!;
  }
  if (key === "End") {
    return order[order.length - 1]!;
  }
  if (key === "ArrowLeft") {
    return order[Math.max(0, at - 1)]!;
  }
  return order[Math.min(order.length - 1, at + 1)]!;
}
