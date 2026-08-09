/**
 * Pure plan/edit proposal kind mapping (A6.147).
 * Maps UI plan-queue kinds onto host EditProposalKind strings.
 */

export function proposalKindFromPlanEdit(
  kind: string,
  isNewFile?: boolean,
): string {
  if (kind === "create_directory") return "create_directory";
  if (isNewFile || kind === "create_file") return "create_file";
  if (kind === "write_file" || kind === "edit" || kind === "multi_edit") {
    return kind;
  }
  return "edit_file";
}
