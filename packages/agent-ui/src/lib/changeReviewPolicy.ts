/**
 * Pure change-review panel mappers (A6.69).
 * Hosts filter edit_diff tool rows into review queue items; no Apply/host I/O.
 */

import { isEditDiffMessage, type EditDiffMessageLike } from "./editDiffMessagePolicy.js";

export type ChangeReviewSourceMessage = EditDiffMessageLike & {
  id: string;
  filePath?: string;
};

export type ChangeReviewItem = {
  id: string;
  path: string;
  kind: string;
  isNewFile: boolean;
  description?: string;
  originalContent: string;
  proposedContent: string;
};

/** Coarse line stats (Desktop/planDiffStats parity — no LCS). */
export function planLineDiffStats(
  original: string,
  proposed: string,
): { added: number; removed: number } {
  const a = original.split("\n");
  const b = proposed.split("\n");
  const setA = new Set(a);
  const setB = new Set(b);
  let added = 0;
  let removed = 0;
  for (const line of b) {
    if (!setA.has(line)) {
      added += 1;
    }
  }
  for (const line of a) {
    if (!setB.has(line)) {
      removed += 1;
    }
  }
  return { added, removed };
}

export function listChangeReviewItems(
  messages: readonly ChangeReviewSourceMessage[],
  dismissedIds: ReadonlySet<string> = new Set(),
): ChangeReviewItem[] {
  const items: ChangeReviewItem[] = [];
  for (const message of messages) {
    if (!isEditDiffMessage(message)) {
      continue;
    }
    if (dismissedIds.has(message.id)) {
      continue;
    }
    const original = message.diffOriginalText ?? "";
    const proposed = message.diffModifiedText ?? "";
    const path = message.filePath?.trim() || "untitled";
    const isNewFile = original.length === 0 && proposed.length > 0;
    items.push({
      id: message.id,
      path,
      kind: isNewFile ? "create_file" : "edit_file",
      isNewFile,
      originalContent: original,
      proposedContent: proposed,
    });
  }
  return items;
}

export function dismissChangeReviewId(
  dismissed: ReadonlySet<string>,
  id: string,
): Set<string> {
  const next = new Set(dismissed);
  next.add(id);
  return next;
}

export function dismissAllChangeReviewIds(
  items: readonly Pick<ChangeReviewItem, "id">[],
): Set<string> {
  return new Set(items.map((item) => item.id));
}
