/**
 * Shared filesystem mutations for plan / edit-proposal Apply and undo.
 * Desktop ReviewPort and planStore both funnel here so Apply is not
 * ad-hoc `native.writeFile` in the UI layer alone.
 */

export type PlanEditFs = {
  writeFile(
    path: string,
    content: string,
    opts?: { source?: string },
  ): Promise<void>;
  createDir(
    path: string,
    opts?: { source?: string },
  ): Promise<void>;
  delete(
    path: string,
    opts?: { source?: string },
  ): Promise<void>;
};

export type PlanEditMutation = {
  kind: string;
  path: string;
  proposedContent: string;
  originalContent?: string;
  isNewFile?: boolean;
};

/** Map plan-queue / proposal kinds onto host EditProposalKind strings. */
export function proposalKindFromPlanEdit(kind: string, isNewFile?: boolean): string {
  if (kind === "create_directory") return "create_directory";
  if (isNewFile || kind === "create_file") return "create_file";
  if (kind === "write_file" || kind === "edit" || kind === "multi_edit") return kind;
  return "edit_file";
}

/**
 * Apply a queued plan edit or host edit proposal to the filesystem.
 */
export async function applyPlanEditMutation(
  fs: PlanEditFs,
  item: PlanEditMutation,
  source = "ai-plan-review",
): Promise<void> {
  const path = item.path.trim();
  if (!path) {
    throw new Error("invalid_proposal_path");
  }
  if (item.kind === "create_directory") {
    await fs.createDir(path, { source });
    return;
  }
  await fs.writeFile(path, item.proposedContent, { source });
}

/**
 * Undo a previously applied plan file edit in this session.
 * Directory creates are not restored here (unsafe once populated).
 */
export async function restorePlanEditMutation(
  fs: PlanEditFs,
  item: PlanEditMutation,
  source = "ai-plan-restore",
): Promise<void> {
  const path = item.path.trim();
  if (!path) {
    throw new Error("invalid_proposal_path");
  }
  if (item.kind === "create_directory") {
    throw new Error("directory_restore_unsupported");
  }
  if (item.isNewFile) {
    await fs.delete(path, { source });
    return;
  }
  await fs.writeFile(path, item.originalContent ?? "", { source });
}
