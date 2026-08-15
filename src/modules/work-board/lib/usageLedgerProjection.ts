import type { WorkAttemptPhase, WorkUsage } from "@altai/host-contract";

/**
 * Usage ledger projection (package 065, PR 2). Every run's token usage is
 * already durable in the agent event journal — the producer has been
 * appending it since the delivery loop landed. This projection joins that
 * ledger to Work through the attempt's chat binding: each row names the
 * Work, the attempt, and the tokens its chat recorded. An attempt that
 * never bound a chat has nothing to attribute — a different fact from a
 * chat that recorded zero, and the labels keep them distinct.
 */

export type UsageLedgerRow = {
  id: string;
  workId: string;
  workTitle: string;
  attemptLabel: string;
  phase: WorkAttemptPhase;
  phaseLabel: string;
  chatId: string | null;
  /** Token totals when the attempt bound a chat; null when it never did. */
  tokens: WorkUsage["usage"];
  tokenLabel: string;
  cacheLabel: string | null;
  atMs: number;
};

export type UsageLedgerSummary = {
  attemptCount: number;
  attributedCount: number;
  unattributedCount: number;
  promptTokens: number;
  completionTokens: number;
  totalTokens: number;
};

function label(value: string): string {
  return value.replace(/_/g, " ");
}

function count(value: number): string {
  return value.toLocaleString("en-US");
}

export function toUsageLedgerRow(usage: WorkUsage): UsageLedgerRow {
  return {
    id: usage.attemptId,
    workId: usage.workId,
    workTitle: usage.workTitle,
    attemptLabel: `Attempt ${usage.number}`,
    phase: usage.phase,
    phaseLabel: label(usage.phase),
    chatId: usage.chatId ?? null,
    tokens: usage.usage ?? null,
    tokenLabel: usage.usage
      ? `${count(usage.usage.totalTokens)} total`
      : "no chat bound",
    cacheLabel:
      usage.usage && usage.usage.cacheReadTokens + usage.usage.cacheCreationTokens > 0
        ? `${count(usage.usage.cacheReadTokens)} cache read · ${count(
            usage.usage.cacheCreationTokens,
          )} cache write`
        : null,
    atMs: usage.updatedAtMs,
  };
}

/** Project the ledger's attempts, newest first. The server already orders
 *  the page; the projection preserves that order. */
export function projectUsageLedger(rows: readonly WorkUsage[]): UsageLedgerRow[] {
  return rows.map(toUsageLedgerRow);
}

/** Sum the page: how many attempts carry attribution, how many never
 *  bound a chat, and the token totals across attributed attempts. */
export function summarizeUsageLedger(
  rows: readonly UsageLedgerRow[],
): UsageLedgerSummary {
  return rows.reduce<UsageLedgerSummary>(
    (summary, row) => {
      if (row.tokens) {
        return {
          ...summary,
          attributedCount: summary.attributedCount + 1,
          promptTokens: summary.promptTokens + row.tokens.promptTokens,
          completionTokens: summary.completionTokens + row.tokens.completionTokens,
          totalTokens: summary.totalTokens + row.tokens.totalTokens,
        };
      }
      return { ...summary, unattributedCount: summary.unattributedCount + 1 };
    },
    {
      attemptCount: rows.length,
      attributedCount: 0,
      unattributedCount: 0,
      promptTokens: 0,
      completionTokens: 0,
      totalTokens: 0,
    },
  );
}
