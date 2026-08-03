import { createAppStore } from "@/lib/appStore";

export type TodoStatus = "pending" | "in_progress" | "completed";

export type Todo = {
  id: string;
  title: string;
  description?: string;
  status: TodoStatus;
  /** Missing on legacy/runtime todos; only explicit board todos use manual. */
  origin?: "agent" | "manual";
};

const STORE_PATH = "altai-ai-todos.json";
const todosKey = (sessionId: string) => `todos:${sessionId}`;

const store = createAppStore(STORE_PATH, { defaults: {}, autoSave: 200 });

export async function loadTodos(sessionId: string): Promise<Todo[]> {
  return (await store.get<Todo[]>(todosKey(sessionId))) ?? [];
}

export async function saveTodos(
  sessionId: string,
  todos: Todo[],
): Promise<void> {
  await store.set(todosKey(sessionId), todos);
}

export async function deleteTodos(sessionId: string): Promise<void> {
  await store.delete(todosKey(sessionId));
}
