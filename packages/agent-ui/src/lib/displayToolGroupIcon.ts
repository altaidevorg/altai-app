/**
 * Pure display tool-group icon keys (A6.56).
 * Hosts map keys to Hugeicons / native glyphs.
 */

import type { DisplayToolGroupKind } from "./displayTranscriptBlocks.js";

export type DisplayToolGroupIconKey = "file" | "terminal" | "globe";

export function displayToolGroupIconKey(
  kind: DisplayToolGroupKind,
): DisplayToolGroupIconKey {
  if (kind === "reads") return "file";
  if (kind === "cmd") return "terminal";
  return "globe";
}
