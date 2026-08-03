import { TodoSummaryChip as TodoSummaryChipView } from "@altai/agent-ui";
import { useEffect } from "react";
import type { Todo } from "../lib/todos";
import { useTodosStore } from "../store/todoStore";

type Props = { sessionId: string | null };

const EMPTY_TODOS: Todo[] = [];

/**
 * Desktop adapter: hydrates session todos from the local store, then renders
 * the shared plan summary chip.
 */
export function TodoSummaryChip({ sessionId }: Props) {
  const hydrate = useTodosStore((s) => s.hydrate);
  const todos =
    useTodosStore((s) => (sessionId ? s.bySession[sessionId] : undefined)) ??
    EMPTY_TODOS;

  useEffect(() => {
    if (sessionId) void hydrate(sessionId);
  }, [sessionId, hydrate]);

  if (!sessionId) return null;
  return <TodoSummaryChipView todos={todos} />;
}
