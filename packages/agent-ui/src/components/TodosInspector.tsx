import { cn } from "../lib/cn.js";
import { InspectorEmpty } from "./InspectorEmpty.js";

export type TodosInspectorItem = {
  id: string;
  title: string;
  status: string;
};

export type TodosInspectorProps = {
  done: number;
  total: number;
  todos: TodosInspectorItem[];
};

/**
 * Plan checklist: progress + divide-y rows (no nested cards).
 */
export function TodosInspector({ done, total, todos }: TodosInspectorProps) {
  if (!total) {
    return (
      <InspectorEmpty>
        The agent’s task checklist will appear here.
      </InspectorEmpty>
    );
  }
  return (
    <div>
      <div className="mb-1.5 flex items-center gap-2 px-0.5 text-[11px] text-muted-foreground">
        <span className="relative h-1 flex-1 overflow-hidden rounded-full bg-muted">
          <span
            className="absolute inset-y-0 left-0 rounded-full bg-foreground/70"
            style={{ width: `${Math.round((done / total) * 100)}%` }}
          />
        </span>
        <span className="tabular-nums">
          {done}/{total}
        </span>
      </div>
      <ul className="divide-y divide-border-subtle">
        {todos.map((todo) => (
          <li key={todo.id} className="flex items-start gap-2 py-2">
            <span
              className={cn(
                "mt-1.5 size-1.5 shrink-0 rounded-full",
                todo.status === "completed"
                  ? "bg-foreground"
                  : todo.status === "in_progress"
                    ? "bg-foreground/70"
                    : "bg-muted-foreground/40",
              )}
            />
            <span
              className={cn(
                "text-[11px] leading-relaxed",
                todo.status === "completed" &&
                  "text-muted-foreground line-through",
              )}
            >
              {todo.title}
            </span>
          </li>
        ))}
      </ul>
    </div>
  );
}
