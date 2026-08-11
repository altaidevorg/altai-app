import type { WorkInboxItem } from "@altai/host-contract";

export const WORK_INBOX_INVALIDATION_EVENTS = [
  "altai:work-inbox-changed",
  "altai:agent-terminal-journaled",
] as const;

export type WorkInboxAttentionSources = {
  reconcile: () => Promise<unknown>;
  list: () => Promise<WorkInboxItem[]>;
};

export type WorkInboxRequestToken = {
  workspacePath: string;
  generation: number;
};

/** Monotonic workspace-scoped guard for poll/event/request races. */
export class WorkInboxRequestGate {
  private workspacePath: string;
  private generation = 0;

  constructor(workspacePath: string) {
    this.workspacePath = workspacePath;
  }

  reset(workspacePath: string): void {
    this.workspacePath = workspacePath;
    this.generation += 1;
  }

  ownsWorkspace(workspacePath: string): boolean {
    return workspacePath === this.workspacePath;
  }

  begin(workspacePath: string): WorkInboxRequestToken {
    if (workspacePath !== this.workspacePath) {
      return { workspacePath, generation: -1 };
    }
    this.generation += 1;
    return { workspacePath, generation: this.generation };
  }

  isCurrent(token: WorkInboxRequestToken): boolean {
    return (
      token.workspacePath === this.workspacePath &&
      token.generation === this.generation
    );
  }
}

/** Reconcile first, then count the canonical Work projection. */
export async function loadWorkInboxAttentionCount(
  sources: WorkInboxAttentionSources,
): Promise<number | null> {
  // A recovery failure must not hide already-persisted review/blocker rows.
  await sources.reconcile().catch(() => undefined);
  try {
    return (await sources.list()).length;
  } catch {
    // Null means "keep the last known count". Only a workspace epoch reset
    // clears it; a transient host failure must not hide attention.
    return null;
  }
}
