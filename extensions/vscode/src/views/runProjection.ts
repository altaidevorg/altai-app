import type { ChatRunEvent } from "./chatMessages.js";

export type RunAttentionKind = "failure" | "warning" | "unsupported_attention";
export type RunPhase = "active" | "completed" | "cancelled" | RunAttentionKind;

/** A stable identity shared by Chat, Work, and Inbox for one host run. */
export type RunIdentity = {
  readonly chatId: string;
  readonly runId: string;
};

export type ProjectedRun = RunIdentity & {
  readonly key: string;
  readonly phase: RunPhase;
  readonly title: string;
  readonly detail: string;
  readonly lastSeq: number;
  readonly attention?: RunAttentionKind;
};

export type RunProjection = {
  readonly active: readonly ProjectedRun[];
  readonly history: readonly ProjectedRun[];
  readonly attention: readonly ProjectedRun[];
};

export type RunDeepLink = {
  readonly command: "altai.revealRun";
  readonly title: "Reveal in Chat";
  readonly arguments: [RunIdentity];
};

type MutableRun = ProjectedRun & { updatedOrder: number };

/**
 * In-memory projection of validated host events. There is deliberately no
 * persistence here: the host has no replay API yet, so a window reload starts
 * with an empty projection rather than pretending historical data is present.
 */
export class RunProjectionStore {
  private readonly runs = new Map<string, MutableRun>();
  private readonly listeners = new Set<(projection: RunProjection) => void>();
  private nextOrder = 0;

  ingest(event: ChatRunEvent): boolean {
    const key = runKey(event);
    const existing = this.runs.get(key);
    if (existing && event.seq <= existing.lastSeq) return false;

    const base: MutableRun = existing ?? {
      key,
      chatId: event.chatId,
      runId: event.runId,
      phase: "active",
      title: "ALTAI is working",
      detail: "Run started",
      lastSeq: 0,
      updatedOrder: 0,
    };
    const next = projectEvent(base, event, ++this.nextOrder);
    this.runs.set(key, next);
    this.notify();
    return true;
  }

  snapshot(): RunProjection {
    const all = [...this.runs.values()].sort((left, right) => right.updatedOrder - left.updatedOrder);
    return {
      active: all.filter((run) => run.phase === "active"),
      history: all.filter((run) => run.phase !== "active"),
      attention: all.filter((run) => run.attention !== undefined),
    };
  }

  subscribe(listener: (projection: RunProjection) => void): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  clear(): void {
    if (this.runs.size === 0) return;
    this.runs.clear();
    this.notify();
  }

  private notify(): void {
    const projection = this.snapshot();
    for (const listener of this.listeners) listener(projection);
  }
}

export function runKey(identity: RunIdentity): string {
  return `${identity.chatId}:${identity.runId}`;
}

/** Tree rows use this command to return to the same Chat/run identity. */
export function chatRunDeepLink(run: RunIdentity): RunDeepLink {
  return { command: "altai.revealRun", title: "Reveal in Chat", arguments: [{ chatId: run.chatId, runId: run.runId }] };
}

function projectEvent(run: MutableRun, event: ChatRunEvent, updatedOrder: number): MutableRun {
  const next = { ...run, lastSeq: event.seq, updatedOrder };
  switch (event.type) {
    case "run_started":
      return { ...next, phase: "active", attention: undefined, title: "ALTAI is working", detail: "Run started" };
    case "thinking":
      return { ...next, phase: "active", title: "ALTAI is thinking", detail: summary(event.content, "Thinking") };
    case "agent_message":
      return { ...next, phase: "active", title: "ALTAI is responding", detail: summary(event.content, "Generating response") };
    case "tool_call_start":
      return { ...next, phase: "active", title: `Using ${event.name}`, detail: "Tool activity" };
    case "run_terminated": {
      const attention = terminalAttention(event.outcome);
      const phase: RunPhase = attention ?? (event.outcome === "completed" ? "completed" : "cancelled");
      return {
        ...next,
        phase,
        ...(attention === undefined ? {} : { attention }),
        title: terminalTitle(event.outcome, attention),
        detail: `Terminal outcome: ${event.outcome}`,
      };
    }
  }
}

function terminalAttention(outcome: string): RunAttentionKind | undefined {
  const value = outcome.toLowerCase();
  if (value === "completed" || value === "cancelled") return undefined;
  if (value.includes("warning")) return "warning";
  // The protocol currently has no approval/steering request. If a terminal
  // outcome advertises a pending user decision, surface it read-only instead
  // of implying that the extension can resolve it.
  if (/(approval|attention|pending|waiting|input_required)/.test(value)) return "unsupported_attention";
  return "failure";
}

function terminalTitle(outcome: string, attention: RunAttentionKind | undefined): string {
  if (attention === "warning") return "Run completed with warnings";
  if (attention === "unsupported_attention") return "Run needs unsupported attention";
  if (attention === "failure") return "Run failed";
  return outcome === "completed" ? "Run completed" : "Run cancelled";
}

function summary(value: string, fallback: string): string {
  const normalized = value.trim().replace(/\s+/g, " ");
  return normalized.length === 0 ? fallback : normalized.slice(0, 140);
}
