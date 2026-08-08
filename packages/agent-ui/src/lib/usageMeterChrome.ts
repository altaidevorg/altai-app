/**
 * Accumulate host usage events into a run token meter (A6.72).
 * Pure — no React / host imports (Desktop agentMeta parity).
 */

export type RunUsageTotals = {
  inputTokens: number;
  outputTokens: number;
  cachedInputTokens: number;
  /** Last total_tokens field from the host when provided; else input+output. */
  totalTokens: number;
};

export const ZERO_RUN_USAGE: RunUsageTotals = {
  inputTokens: 0,
  outputTokens: 0,
  cachedInputTokens: 0,
  totalTokens: 0,
};

export type UsageDelta = {
  promptTokens: number;
  completionTokens: number;
  cacheReadTokens: number;
  totalTokens: number;
};

function readNonnegNumber(value: unknown): number | null {
  if (typeof value !== "number" || !Number.isFinite(value) || value < 0) {
    return null;
  }
  return Math.floor(value);
}

/**
 * Parse a mapped AgentEvent payload (or nested stdio event object) for usage.
 * Returns null when the payload is not a usage sample.
 */
export function usageDeltaFromPayload(payload: unknown): UsageDelta | null {
  if (!payload || typeof payload !== "object" || Array.isArray(payload)) {
    return null;
  }
  const record = payload as Record<string, unknown>;
  // Nested: payload.event.payload or entire event as body.
  const body =
    record.type === "usage"
      ? record
      : record.payload &&
          typeof record.payload === "object" &&
          !Array.isArray(record.payload) &&
          (record.payload as { type?: unknown }).type === "usage"
        ? (record.payload as Record<string, unknown>)
        : record;

  if (typeof body.type === "string" && body.type !== "usage") {
    // Explicit non-usage types reject; missing type still tries token fields.
    if (
      body.type === "message" ||
      body.type === "tool" ||
      body.type === "lifecycle" ||
      body.type === "thinking"
    ) {
      return null;
    }
  }

  const prompt =
    readNonnegNumber(body.prompt_tokens) ??
    readNonnegNumber(body.promptTokens);
  const completion =
    readNonnegNumber(body.completion_tokens) ??
    readNonnegNumber(body.completionTokens);
  const cacheRead =
    readNonnegNumber(body.cache_read_tokens) ??
    readNonnegNumber(body.cacheReadTokens) ??
    0;
  const totalField =
    readNonnegNumber(body.total_tokens) ?? readNonnegNumber(body.totalTokens);

  if (prompt === null && completion === null && totalField === null) {
    return null;
  }

  const promptTokens = prompt ?? 0;
  const completionTokens = completion ?? 0;
  const totalTokens = totalField ?? promptTokens + completionTokens;

  return {
    promptTokens,
    completionTokens,
    cacheReadTokens: cacheRead,
    totalTokens,
  };
}

/** Accumulate one host usage sample into run-level totals. */
export function accumulateRunUsage(
  current: RunUsageTotals,
  delta: UsageDelta,
): RunUsageTotals {
  const inputTokens = current.inputTokens + delta.promptTokens;
  const outputTokens = current.outputTokens + delta.completionTokens;
  const cachedInputTokens = current.cachedInputTokens + delta.cacheReadTokens;
  // Prefer summing reported totals when present on the sample; else recomputed.
  const sampleTotal =
    delta.totalTokens > 0
      ? delta.totalTokens
      : delta.promptTokens + delta.completionTokens;
  return {
    inputTokens,
    outputTokens,
    cachedInputTokens,
    totalTokens: current.totalTokens + sampleTotal,
  };
}

export function formatRunTokenLabel(input: {
  hasActiveRun: boolean;
  status: "idle" | "running" | "blocked";
  usage: RunUsageTotals | null;
}): string {
  if (input.status === "blocked") {
    if (input.usage && input.usage.totalTokens > 0) {
      return `Tokens · ${formatTokenCount(input.usage.totalTokens)}`;
    }
    return "Tokens · n/a";
  }
  if (input.usage && input.usage.totalTokens > 0) {
    return `Tokens · ${formatTokenCount(input.usage.totalTokens)}`;
  }
  if (input.hasActiveRun) {
    return "Tokens · live";
  }
  return "Tokens · —";
}

/** Compact integer (e.g. 1200 → 1.2k) for the status chip. */
export function formatTokenCount(n: number): string {
  if (!Number.isFinite(n) || n < 0) {
    return "0";
  }
  if (n < 1000) {
    return String(Math.floor(n));
  }
  if (n < 10_000) {
    const tenths = Math.round(n / 100) / 10;
    return `${tenths}k`.replace(/\.0k$/, "k");
  }
  if (n < 1_000_000) {
    return `${Math.round(n / 1000)}k`;
  }
  const mills = Math.round(n / 100_000) / 10;
  return `${mills}M`.replace(/\.0M$/, "M");
}
