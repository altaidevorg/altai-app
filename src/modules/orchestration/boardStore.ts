/**
 * Task Board store (C1) — connects the orchestration task graph,
 * quality metrics, and usage tracking to a reactive Zustand store.
 */

import { create } from "zustand";
import {
  native,
  type AttemptAnalysis,
  type AttemptOutcome,
} from "@/modules/ai/lib/native";

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

function columnForOutcome(outcome: AttemptOutcome): BoardColumn {
  switch (outcome) {
    case "success":
      return "done";
    case "expensive":
      return "reviewing";
    case "failure":
    case "abandoned":
      return "blocked";
  }
}

function priorityForAnalysis(analysis: AttemptAnalysis): BoardTask["priority"] {
  if (analysis.outcome === "failure" || analysis.attemptCount >= 4) {
    return "critical";
  }
  if (analysis.outcome === "expensive" || analysis.attemptCount >= 2) {
    return "high";
  }
  if (analysis.outcome === "abandoned") return "low";
  return "normal";
}

function tasksFromAnalyses(analyses: AttemptAnalysis[]): BoardTask[] {
  return analyses.map((analysis) => ({
    taskId: analysis.taskId,
    title: analysis.taskId,
    column: columnForOutcome(analysis.outcome),
    attemptCount: analysis.attemptCount,
    blockedReason:
      analysis.outcome === "failure" || analysis.outcome === "abandoned"
        ? analysis.signals.map((signal) => signal.detail).filter(Boolean)
        : null,
    priority: priorityForAnalysis(analysis),
  }));
}

export const useBoardStore = create<BoardState>((set) => ({
  tasks: [],
  loading: false,
  error: null,
  metrics: null,

  load: async (workspaceKey: string, dbPath: string) => {
    set({ loading: true, error: null });
    try {
      const [metrics, analyses] = await Promise.all([
        native.orchestrationQualityMetrics(
          dbPath,
          workspaceKey,
          86_400_000,
        ),
        native.orchestrationSessionAnalyze(dbPath, workspaceKey),
      ]);
      set({
        loading: false,
        tasks: tasksFromAnalyses(analyses),
        metrics: {
          firstAttemptSuccessRate: metrics.firstAttemptSuccessRate,
          retryRate: metrics.retryRate,
          medianTimeToHandoffMs: metrics.medianTimeToHandoffMs,
        },
      });
    } catch (e) {
      console.error("Task board load failed:", e);
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
