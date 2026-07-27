/**
 * Task Board store (C1) — connects the orchestration task graph,
 * quality metrics, and usage tracking to a reactive Zustand store.
 */

import { create } from "zustand";
import { native } from "@/modules/ai/lib/native";

export type BoardColumn = "queued" | "running" | "reviewing" | "done" | "blocked";

export type BoardTask = {
  taskId: string;
  title: string;
  column: BoardColumn;
  attemptCount: number;
  blockedReason: string[] | null;
  priority: "low" | "normal" | "high" | "critical";
};

type BoardState = {
  tasks: BoardTask[];
  loading: boolean;
  error: string | null;
  metrics: {
    firstAttemptSuccessRate: number | null;
    retryRate: number | null;
    medianTimeToHandoffMs: number | null;
  } | null;
  load: (workspaceKey: string, dbPath: string) => Promise<void>;
  moveTask: (taskId: string, column: BoardColumn) => void;
  clear: () => void;
};

export const useBoardStore = create<BoardState>((set) => ({
  tasks: [],
  loading: false,
  error: null,
  metrics: null,

  load: async (workspaceKey: string, dbPath: string) => {
    set({ loading: true, error: null });
    try {
      const metrics = await native.orchestrationQualityMetrics(
        dbPath,
        workspaceKey,
        86_400_000,
      );
      set({
        loading: false,
        metrics: {
          firstAttemptSuccessRate: metrics.firstAttemptSuccessRate,
          retryRate: metrics.retryRate,
          medianTimeToHandoffMs: metrics.medianTimeToHandoffMs,
        },
      });
    } catch (e) {
      set({ loading: false, error: String(e) });
    }
  },

  moveTask: (taskId, column) =>
    set((state) => ({
      tasks: state.tasks.map((t) =>
        t.taskId === taskId ? { ...t, column } : t,
      ),
    })),

  clear: () => set({ tasks: [], metrics: null, error: null }),
}));
