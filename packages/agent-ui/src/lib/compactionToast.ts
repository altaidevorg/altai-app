/**
 * Pure compaction activity toast copy (A6.202).
 */

export function compactionRequestedLabel(): string {
  return "Context compaction requested";
}

export function compactionRequestedDetail(): string {
  return "Queued directly on the agent runtime";
}

export function compactionFailedLabel(): string {
  return "Context compaction failed";
}

export function compactionFailedDetail(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
