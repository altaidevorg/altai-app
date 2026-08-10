import { create } from "zustand";
import {
  errorMessageFromUnknown,
  indexLatestCronJobsByAutomationId,
  normalizedWorkspacePath as normalizedWorkspacePathShared,
  sortAutomationItemsById,
} from "@altai/agent-ui";
import {
  native,
  type AgentBackgroundJobInfo,
  type AgentAutomationInfo,
  type AgentAutomationSchedule,
} from "../lib/native";

type DirectSchedule = Extract<
  AgentAutomationSchedule,
  { kind: "at" | "every" }
>;

type AutomationState = {
  workspacePath: string | null;
  items: AgentAutomationInfo[];
  jobsByAutomationId: Record<string, AgentBackgroundJobInfo>;
  hydrated: boolean;
  loading: boolean;
  error: string | null;
  pendingIds: Record<string, true>;
  refresh: (workspacePath?: string | null) => Promise<void>;
  create: (
    chatId: string,
    schedule: DirectSchedule,
    message: string,
  ) => Promise<boolean>;
  remove: (automationId: string, chatId: string) => Promise<void>;
  clearError: () => void;
};

function normalizedWorkspacePath(path?: string | null): string | null {
  return normalizedWorkspacePathShared(path);
}

function messageFrom(error: unknown): string {
  return errorMessageFromUnknown(error);
}


export const useAutomationStore = create<AutomationState>((set, get) => ({
  workspacePath: null,
  items: [],
  jobsByAutomationId: {},
  hydrated: false,
  loading: false,
  error: null,
  pendingIds: {},

  refresh: async (workspacePath) => {
    const path =
      workspacePath === undefined
        ? get().workspacePath
        : normalizedWorkspacePath(workspacePath);
    if (!path) {
      set({
        workspacePath: null,
        items: [],
        jobsByAutomationId: {},
        hydrated: true,
        loading: false,
        error: null,
        pendingIds: {},
      });
      return;
    }
    set({
      workspacePath: path,
      loading: true,
      error: null,
      ...(get().workspacePath === path
        ? {}
        : { items: [], jobsByAutomationId: {}, hydrated: false, pendingIds: {} }),
    });
    try {
      const [items, jobs] = await Promise.all([
        native.agentListAutomations(path),
        native.agentListBackgroundJobs({ workspacePath: path, limit: 200 }),
      ]);
      const jobsByAutomationId = indexLatestCronJobsByAutomationId(jobs);
      if (get().workspacePath === path) {
        set({ items: sortAutomationItemsById(items), jobsByAutomationId, hydrated: true, loading: false });
      }
    } catch (error) {
      if (get().workspacePath === path) {
        set({ hydrated: true, loading: false, error: messageFrom(error) });
      }
    }
  },

  create: async (chatId, schedule, message) => {
    const path = get().workspacePath;
    if (!path) {
      set({ error: "Open a workspace before creating an automation." });
      return false;
    }
    const pendingKey = "create";
    set((state) => ({
      error: null,
      pendingIds: { ...state.pendingIds, [pendingKey]: true },
    }));
    try {
      const item = await native.agentAutomationCreate(chatId, schedule, message, path);
      set((state) => ({ items: sortItems([...state.items, item]) }));
      return true;
    } catch (error) {
      set({ error: messageFrom(error) });
      return false;
    } finally {
      set((state) => {
        const pendingIds = { ...state.pendingIds };
        delete pendingIds[pendingKey];
        return { pendingIds };
      });
    }
  },

  remove: async (automationId, chatId) => {
    const path = get().workspacePath;
    if (!path) return;
    const pendingKey = `remove:${automationId}`;
    set((state) => ({
      error: null,
      pendingIds: { ...state.pendingIds, [pendingKey]: true },
    }));
    try {
      await native.agentAutomationRemove(automationId, chatId, path);
      set((state) => ({
        items: state.items.filter((item) => item.id !== automationId),
        jobsByAutomationId: Object.fromEntries(
          Object.entries(state.jobsByAutomationId).filter(([id]) => id !== automationId),
        ),
      }));
    } catch (error) {
      set({ error: messageFrom(error) });
    } finally {
      set((state) => {
        const pendingIds = { ...state.pendingIds };
        delete pendingIds[pendingKey];
        return { pendingIds };
      });
    }
  },

  clearError: () => set({ error: null }),
}));
