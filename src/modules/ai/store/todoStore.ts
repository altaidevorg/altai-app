import { create } from "zustand";
import {
  deleteTodos as persistDelete,
  loadTodos as persistLoad,
  saveTodos as persistSave,
  type Todo,
} from "../lib/todos";

type TodosState = {
  /** Map of sessionId -> todos. */
  bySession: Record<string, Todo[]>;
  /** Set of sessionIds whose todos were hydrated. */
  hydrated: Set<string>;
  hydrate: (sessionId: string) => Promise<void>;
  setTodos: (sessionId: string, todos: Todo[]) => void;
  addTodo: (
    sessionId: string,
    input: { title: string; description?: string },
  ) => Todo;
  updateTodoStatus: (
    sessionId: string,
    todoId: string,
    status: Todo["status"],
  ) => void;
  clearSession: (sessionId: string) => Promise<void>;
};

export const useTodosStore = create<TodosState>((set, get) => ({
  bySession: {},
  hydrated: new Set(),

  async hydrate(sessionId) {
    if (get().hydrated.has(sessionId)) return;
    const todos = await persistLoad(sessionId);
    set((s) => {
      const nextHydrated = new Set(s.hydrated);
      nextHydrated.add(sessionId);
      // A setTodos() (e.g. a todo_write ingest) may have populated this session
      // while persistLoad was awaited — don't clobber the live plan with disk.
      if (s.bySession[sessionId] !== undefined) {
        return { hydrated: nextHydrated };
      }
      return {
        bySession: { ...s.bySession, [sessionId]: todos },
        hydrated: nextHydrated,
      };
    });
  },

  setTodos(sessionId, todos) {
    // todo_write replaces the agent's current plan, but user-created board
    // todos are durable work items and must survive those plan refreshes.
    const manual = (get().bySession[sessionId] ?? []).filter(
      (todo) => todo.origin === "manual",
    );
    const next = [
      ...manual,
      ...todos
        .filter((todo) => todo.origin !== "manual")
        .map((todo) => ({ ...todo, origin: "agent" as const })),
    ];
    set((state) => ({
      bySession: { ...state.bySession, [sessionId]: next },
    }));
    void persistSave(sessionId, next);
  },

  addTodo(sessionId, input) {
    const todo: Todo = {
      id: `todo-${Date.now().toString(36)}-${Math.random()
        .toString(36)
        .slice(2, 7)}`,
      title: input.title.trim(),
      description: input.description?.trim() || undefined,
      status: "pending",
      origin: "manual",
    };
    const todos = [...(get().bySession[sessionId] ?? []), todo];
    set((state) => ({
      bySession: { ...state.bySession, [sessionId]: todos },
    }));
    void persistSave(sessionId, todos);
    return todo;
  },

  updateTodoStatus(sessionId, todoId, status) {
    const current = get().bySession[sessionId] ?? [];
    const next = current.map((todo) =>
      todo.id === todoId ? { ...todo, status } : todo,
    );
    set((state) => ({
      bySession: { ...state.bySession, [sessionId]: next },
    }));
    void persistSave(sessionId, next);
  },

  async clearSession(sessionId) {
    set((s) => {
      const next = { ...s.bySession };
      delete next[sessionId];
      const nextHydrated = new Set(s.hydrated);
      nextHydrated.delete(sessionId);
      return { bySession: next, hydrated: nextHydrated };
    });
    await persistDelete(sessionId);
  },
}));

export function getTodos(sessionId: string | null): Todo[] {
  if (!sessionId) return [];
  return useTodosStore.getState().bySession[sessionId] ?? [];
}
