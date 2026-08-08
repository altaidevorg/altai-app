/**
 * Pure composer empty-home affordance hints (A6.60).
 * Hosts render the line under the transcript empty state.
 */

export type ComposerHint = {
  key: string;
  glyph: string;
  label: string;
};

export function listComposerAffordances(): readonly ComposerHint[] {
  return [
    { key: "slash", glyph: "/", label: "commands" },
    { key: "snippet", glyph: "#", label: "snippets" },
    { key: "file", glyph: "@", label: "files" },
  ];
}

export function formatComposerHintLine(
  hints: readonly ComposerHint[] = listComposerAffordances(),
): string {
  return hints.map((h) => `${h.glyph} ${h.label}`).join(" · ");
}
