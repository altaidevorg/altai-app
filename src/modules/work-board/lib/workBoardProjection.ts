import type {
  WorkAttempt,
  WorkAttemptPhase,
  WorkInboxItem,
  WorkInboxKind,
  WorkItem,
  WorkState,
} from "@altai/host-contract";

/**
 * Work board projection (package 062). The board's invariant is package
 * 062's gate: **status, execution phase, and attention remain distinct** —
 * `status` is the Work state machine, `executionPhase` is the latest
 * attempt's phase, `attention` is the inbox condition. They are separate
 * typed fields end to end, never folded into one label, because each axis
 * moves independently (a backlog item needs no attempt; a running attempt
 * on in_review work is not the same as review attention).
 */

/** One attention kind per work, highest priority first when several compete. */
export const ATTENTION_PRIORITY: readonly WorkInboxKind[] = [
  "approval",
  "review_required",
  "failed_attempt",
  "blocked",
  "question",
];

/** Board columns in a fixed order; only non-empty columns render. */
export const BOARD_COLUMNS: readonly WorkState[] = [
  "backlog",
  "ready",
  "in_progress",
  "in_review",
  "done",
  "cancelled",
];

export type WorkBoardRow = {
  id: string;
  title: string;
  /** Parent Work's id; the graph's edge source (package 068). */
  parentWorkId: string | null;
  /** Work lifecycle state — the board's column axis. */
  status: WorkState;
  statusLabel: string;
  /** Latest attempt's phase; null when the Work has never been executed. */
  executionPhase: WorkAttemptPhase | null;
  phaseLabel: string | null;
  /** Inbox condition on this Work; null when nothing needs the user. */
  attention: WorkInboxKind | null;
  attentionLabel: string | null;
  updatedLabel: string;
};

function label(value: string): string {
  return value.replace(/_/g, " ");
}

/** The highest-numbered attempt per work, ties broken by updated time. */
export function latestAttemptByWork(
  attempts: readonly WorkAttempt[],
): Map<string, WorkAttempt> {
  const latest = new Map<string, WorkAttempt>();
  for (const attempt of attempts) {
    const current = latest.get(attempt.workId);
    if (
      !current ||
      attempt.number > current.number ||
      (attempt.number === current.number &&
        attempt.updatedAtMs > current.updatedAtMs)
    ) {
      latest.set(attempt.workId, attempt);
    }
  }
  return latest;
}

/** One attention kind per work id, by priority over arrival order. */
export function attentionByWork(
  inbox: readonly WorkInboxItem[],
): Map<string, WorkInboxKind> {
  const rank = new Map(ATTENTION_PRIORITY.map((kind, index) => [kind, index]));
  const attention = new Map<string, WorkInboxKind>();
  for (const item of inbox) {
    const current = attention.get(item.workId);
    if (!current || (rank.get(item.kind) ?? 0) < (rank.get(current) ?? 0)) {
      attention.set(item.workId, item.kind);
    }
  }
  return attention;
}

export function toWorkBoardRow(input: {
  work: WorkItem;
  attempt: WorkAttempt | null;
  attention: WorkInboxKind | null;
  formatUpdated?: (updatedAtMs: number) => string;
}): WorkBoardRow {
  const { work, attempt, attention } = input;
  return {
    id: work.id,
    title: work.title,
    parentWorkId: work.parentWorkId ?? null,
    status: work.state,
    statusLabel: label(work.state),
    executionPhase: attempt ? attempt.phase : null,
    phaseLabel: attempt ? label(attempt.phase) : null,
    attention,
    attentionLabel: attention ? label(attention) : null,
    updatedLabel: (input.formatUpdated ?? defaultFormatUpdated)(work.updatedAtMs),
  };
}

function defaultFormatUpdated(updatedAtMs: number): string {
  return new Date(updatedAtMs).toISOString();
}

/** Compose the three server projections (work, attempts, inbox) into rows,
 *  newest work first. */
export function projectWorkBoard(input: {
  work: readonly WorkItem[];
  attempts: readonly WorkAttempt[];
  inbox: readonly WorkInboxItem[];
  formatUpdated?: (updatedAtMs: number) => string;
}): WorkBoardRow[] {
  const latest = latestAttemptByWork(input.attempts);
  const attention = attentionByWork(input.inbox);
  return [...input.work]
    .sort((a, b) => b.updatedAtMs - a.updatedAtMs)
    .map((work) =>
      toWorkBoardRow({
        work,
        attempt: latest.get(work.id) ?? null,
        attention: attention.get(work.id) ?? null,
        formatUpdated: input.formatUpdated,
      }),
    );
}
