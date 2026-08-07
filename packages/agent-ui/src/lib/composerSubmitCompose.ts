/**
 * Pure composer → agent prompt assembly (Desktop submission markers).
 * Hosts supply file attachments + expanded snippet blocks; no React/store.
 */

import type { ComposerFileAttachment } from "./composerAttachments.js";
import type { ComposerSnippet } from "./composerSnippets.js";
import { expandSnippetTokens } from "./composerSnippets.js";

/** Ordered text blocks for non-binary file context (Desktop wire markers). */
export function formatComposerFileBlocks(
  files: readonly Pick<
    ComposerFileAttachment,
    "kind" | "name" | "mediaType" | "text" | "source"
  >[],
): string[] {
  const fileBlocks = files
    .filter((f) => f.kind === "text")
    .map(
      (f) =>
        `<file name="${f.name}" mediaType="${f.mediaType}">\n${f.text ?? ""}\n</file>`,
    );
  const selectionBlocks = files
    .filter((f) => f.kind === "selection")
    .map(
      (f) =>
        `<selection source="${f.source ?? "terminal"}">\n${f.text ?? ""}\n</selection>`,
    );
  const terminalBlocks = files
    .filter((f) => f.kind === "terminal")
    .map(
      (f) =>
        `<terminal-context name="${f.name}">\n${f.text ?? ""}\n</terminal-context>`,
    );
  const diffBlocks = files
    .filter((f) => f.kind === "diff")
    .map((f) => `<git-diff name="${f.name}">\n${f.text ?? ""}\n</git-diff>`);
  const folderBlocks = files
    .filter((f) => f.kind === "folder")
    .map((f) => `<folder name="${f.name}">\n${f.text ?? ""}\n</folder>`);
  return [
    ...selectionBlocks,
    ...terminalBlocks,
    ...diffBlocks,
    ...folderBlocks,
    ...fileBlocks,
  ];
}

/**
 * Merge picked-chip snippets with token-expanded blocks; first handle wins.
 */
export function mergeSnippetBlocks(input: {
  picked: readonly ComposerSnippet[];
  tokenBlocks: readonly string[];
}): string[] {
  const seenHandles = new Set<string>();
  const allSnippetBlocks: string[] = [];
  for (const s of input.picked) {
    if (seenHandles.has(s.handle)) continue;
    seenHandles.add(s.handle);
    allSnippetBlocks.push(
      `<snippet name="${s.handle}">\n${s.content}\n</snippet>`,
    );
  }
  for (const block of input.tokenBlocks) {
    const m = block.match(/^<snippet name="([^"]+)"/);
    const handle = m?.[1];
    if (handle && seenHandles.has(handle)) continue;
    if (handle) seenHandles.add(handle);
    allSnippetBlocks.push(block);
  }
  return allSnippetBlocks;
}

/**
 * Build the desktop agent prompt: command marker + snippets + file context + body.
 */
export function composeComposerSubmitText(input: {
  commandMarker?: string | null;
  effectiveText: string;
  catalog: readonly ComposerSnippet[];
  pickedSnippets?: readonly ComposerSnippet[];
  files?: readonly Pick<
    ComposerFileAttachment,
    "kind" | "name" | "mediaType" | "text" | "source"
  >[];
}): string {
  const picked = input.pickedSnippets ?? [];
  const files = input.files ?? [];
  const { body: bodyAfterTokens, blocks: snippetBlocks } = expandSnippetTokens(
    input.effectiveText,
    input.catalog,
  );
  const allSnippetBlocks = mergeSnippetBlocks({
    picked,
    tokenBlocks: snippetBlocks,
  });
  return [
    input.commandMarker ?? "",
    allSnippetBlocks.join("\n\n"),
    formatComposerFileBlocks(files).join("\n\n"),
    bodyAfterTokens,
  ]
    .filter(Boolean)
    .join("\n\n");
}

export type ComposerMultimodalParts = {
  imageUrls: string[];
  documents: Array<{ data: string; mediaType: string; name: string }>;
};

/** Extract image URLs and PDF document parts for multimodal send. */
export function extractComposerMultimodalParts(
  files: readonly Pick<
    ComposerFileAttachment,
    "kind" | "url" | "mediaType" | "name"
  >[],
): ComposerMultimodalParts {
  const imageUrls = files
    .filter((f) => f.kind === "image" && f.url)
    .map((f) => f.url as string);
  const documents = files
    .filter((f) => f.kind === "pdf" && f.url)
    .map((f) => ({
      data: f.url!.slice(f.url!.indexOf(",") + 1),
      mediaType: f.mediaType,
      name: f.name,
    }));
  return { imageUrls, documents };
}
