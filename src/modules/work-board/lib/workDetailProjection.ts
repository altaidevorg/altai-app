import type {
  WorkAttempt,
  WorkAttemptPhase,
  WorkInboxItem,
  WorkInboxKind,
  WorkItem,
  WorkState,
} from "@altai/host-contract";

/**
 * Work detail and graph projections (package 062, PR 2). The board's gate —
 * status, execution phase, and attention remain distinct — carries into the
 * detail surface: the model exposes them as separate typed values, and the
 * graph section keeps each related Work's status as its own label instead of
 * folding it into the title.
 */

function label(value: string): string {
  return value.replace(/_/g, " ");
}

export type WorkDetailAttemptRow = {
  id: string;
  number: number;
  phaseLabel: string;
};

export type WorkDetailModel = {
  id: string;
  title: string;
  description: string;
  acceptanceCriteria: string;
  blocker: string | null;
  /** Work lifecycle state. */
  status: WorkState;
  statusLabel: string;
  /** Latest attempt's phase; null when the Work has never been executed. */
  latestPhase: WorkAttemptPhase | null;
  /** The inbox condition on this Work; null when nothing needs the user. */
  attention: WorkInboxKind | null;
  attentionLabel: string | null;
  attemptRows: WorkDetailAttemptRow[];
  revision: number;
  createdAtMs: number;
  updatedAtMs: number;
};

export type WorkGraphRef = {
  id: string;
  title: string;
  stateLabel: string;
};

export type WorkGraphModel = {
  parent: WorkGraphRef | null;
  children: WorkGraphRef[];
};

export function toWorkDetailModel(input: {
  work: WorkItem;
  attempts: readonly WorkAttempt[];
  inbox: readonly WorkInboxItem[];
}): WorkDetailModel {
  const { work, attempts, inbox } = input;
  const sorted = [...attempts].sort((a, b) => b.number - a.number);
  const latest = sorted[0] ?? null;
  const attention =
    inbox.find((item) => item.workId === work.id)?.kind ?? null;
  return {
    id: work.id,
    title: work.title,
    description: work.description,
    acceptanceCriteria: work.acceptanceCriteria,
    blocker: work.blocker ?? null,
    status: work.state,
    statusLabel: label(work.state),
    latestPhase: latest ? latest.phase : null,
    attention,
    attentionLabel: attention ? label(attention) : null,
    attemptRows: sorted.map((attempt) => ({
      id: attempt.id,
      number: attempt.number,
      phaseLabel: label(attempt.phase),
    })),
    revision: work.revision,
    createdAtMs: work.createdAtMs,
    updatedAtMs: work.updatedAtMs,
  };
}

export function toWorkGraphModel(input: {
  work: WorkItem;
  parent: WorkItem | null;
  children: readonly WorkItem[];
}): WorkGraphModel {
  const { parent, children } = input;
  return {
    // A parent id that no longer resolves is "no parent", never an unknown
    // placeholder row.
    parent: parent
      ? {
          id: parent.id,
          title: parent.title,
          stateLabel: label(parent.state),
        }
      : null,
    children: [...children]
      .sort((a, b) => a.createdAtMs - b.createdAtMs)
      .map((child) => ({
        id: child.id,
        title: child.title,
        stateLabel: label(child.state),
      })),
  };
}
