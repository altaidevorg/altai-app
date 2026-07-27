/**
 * Run Inspector store (C2) — event replay, session analysis,
 * and support bundle export.
 */

import { create } from "zustand";
import {
  native,
  type AttemptAnalysis,
  type SupportBundle,
} from "@/modules/ai/lib/native";

type EventEntry = {
  eventId: string;
  taskId: string;
  seq: number;
  kind: string;
  payload: unknown;
  recordedAtMs: number;
};

type InspectorState = {
  events: EventEntry[];
  analyses: AttemptAnalysis[];
  bundle: SupportBundle | null;
  loading: boolean;
  error: string | null;
  selectedTaskId: string | null;
  loadAnalysis: (dbPath: string, workspaceKey: string) => Promise<void>;
  exportBundle: (
    dbPath: string,
    workspaceKey: string,
    taskIds: string[],
  ) => Promise<void>;
  selectTask: (taskId: string | null) => void;
  clear: () => void;
};

export const useInspectorStore = create<InspectorState>((set) => ({
  events: [],
  analyses: [],
  bundle: null,
  loading: false,
  error: null,
  selectedTaskId: null,

  loadAnalysis: async (dbPath, workspaceKey) => {
    set({ loading: true, error: null });
    try {
      const analyses = await native.orchestrationSessionAnalyze(
        dbPath,
        workspaceKey,
      );
      set({ analyses, loading: false });
    } catch (e) {
      console.error("Run inspector analysis failed:", e);
      set({ loading: false, error: String(e) });
    }
  },

  exportBundle: async (dbPath, workspaceKey, taskIds) => {
    set({ loading: true, error: null });
    try {
      // IPC always redacts secrets; unsanitized export is Rust-only.
      const bundle = await native.orchestrationSupportBundle(
        dbPath,
        workspaceKey,
        taskIds,
        "inspector",
      );
      set({ bundle, loading: false });
    } catch (e) {
      console.error("Support bundle export failed:", e);
      set({ loading: false, error: String(e) });
    }
  },

  selectTask: (taskId) => set({ selectedTaskId: taskId }),
  clear: () =>
    set({
      events: [],
      analyses: [],
      bundle: null,
      error: null,
      selectedTaskId: null,
    }),
}));
