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

const RESULT_SUMMARY_FIELDS = ["failure", "error", "message", "summary"] as const;
const RESULT_SUMMARY_MAX = 200;

/** Extract the human-readable line from an attempt's recorded result.
 *  The store writes free-form JSON (failure results, journal outcomes);
 *  anything without a readable field degrades to null, never raw JSON. */
function resultSummary(resultJson: string | null | undefined): string | null {
  if (!resultJson) return null;
  let parsed: unknown;
  try {
    parsed = JSON.parse(resultJson);
  } catch {
    return null;
  }
  if (typeof parsed !== "object" || parsed === null) return null;
  const record = parsed as Record<string, unknown>;
  for (const field of RESULT_SUMMARY_FIELDS) {
    const value = record[field];
    if (typeof value === "string" && value.trim().length > 0) {
      const trimmed = value.trim();
      return trimmed.length > RESULT_SUMMARY_MAX
        ? `${trimmed.slice(0, RESULT_SUMMARY_MAX - 1)}…`
        : trimmed;
    }
  }
  return null;
}

export type WorkDetailAttemptRow = {
  id: string;
  number: number;
  phaseLabel: string;
  /** The attempt's recorded result — its evidence — or null when the
   *  attempt has not recorded one. */
  resultSummary: string | null;
  /** The chat this attempt's transcript lives in; null when unbound. */
  chatId: string | null;
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
      resultSummary: resultSummary(attempt.resultJson),
      chatId: attempt.chatId ?? null,
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
