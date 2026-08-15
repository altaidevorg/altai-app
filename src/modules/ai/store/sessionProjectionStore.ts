import { create } from "zustand";

export type SessionProjection = {
  seq: number;
  timestampRfc3339: string;
  runStatus: string;
  todos: Record<string, unknown>[];
  subagents: Record<string, unknown>[];
  jobs: Record<string, unknown>[];
};

type State = {
  bySession: Record<string, SessionProjection>;
  /** Apply only a strictly newer server snapshot for this session. */
  apply: (sessionId: string, projection: SessionProjection) => boolean;
  clear: (sessionId: string) => void;
};

/**
 * Authoritative IsanAgent session snapshots. This keeps jobs and any future
 * projection fields available without reconstructing them from tool calls.
 */
export const useSessionProjectionStore = create<State>((set) => ({
  bySession: {},
  apply: (sessionId, projection) => {
    let accepted = false;
    set((state) => {
      const current = state.bySession[sessionId];
      if (current && projection.seq <= current.seq) return state;
      accepted = true;
      return {
        bySession: { ...state.bySession, [sessionId]: projection },
      };
    });
    return accepted;
  },
  clear: (sessionId) =>
    set((state) => {
      if (!(sessionId in state.bySession)) return state;
      const next = { ...state.bySession };
      delete next[sessionId];
      return { bySession: next };
    }),
}));
