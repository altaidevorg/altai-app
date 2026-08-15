import type { AuditEvent } from "@altai/host-contract";
import { toWorkTimeline } from "./runTimelineProjection";

/**
 * Audit feed projection (package 065, PR 1). The store's transition log —
 * already the per-Work timeline (package 063) — becomes a workspace-wide
 * feed: every recorded decision, stop, and transition, each row naming
 * the Work it happened to and staying drillable into that Work. Labels
 * reuse the timeline's kind vocabulary so one vocabulary audits both.
 */

export type AuditFeedRow = {
  id: number;
  workId: string;
  workTitle: string;
  label: string;
  detail: string | null;
  atMs: number;
};

/** Project the workspace's recent events (newest first, as the store
 *  returns them) into audit rows, preserving order. */
export function projectAuditFeed(events: readonly AuditEvent[]): AuditFeedRow[] {
  // Same events, same order: zip the timeline rows (label/detail/atMs)
  // with their source events (workId/workTitle).
  const timeline = toWorkTimeline(events);
  return events.map((event, index) => {
    const row = timeline[index];
    return {
      id: event.id,
      workId: event.workId,
      workTitle: event.workTitle,
      label: row.label,
      detail: row.detail,
      atMs: event.createdAtMs,
    };
  });
}
