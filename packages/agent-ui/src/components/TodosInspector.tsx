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
 * Run-inspector todos panel: progress bar + checklist rows. Purely
 * presentational; the host supplies counts and todo items.
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
    <div className="space-y-2">
      <div className="flex items-center gap-2 px-1 text-[10.5px] text-muted-foreground">
        <span className="relative h-1.5 flex-1 overflow-hidden rounded-full bg-muted">
          <span
            className="absolute inset-y-0 left-0 rounded-full bg-foreground/70"
            style={{ width: `${Math.round((done / total) * 100)}%` }}
          />
        </span>
        <span className="tabular-nums">
          {done}/{total}
        </span>
      </div>
      {todos.map((todo) => (
        <div
          key={todo.id}
          className="flex items-start gap-2 rounded-md border border-border bg-muted/30 px-2.5 py-2"
        >
          <span
            className={cn(
              "mt-1 size-1.5 shrink-0 rounded-full",
              todo.status === "completed"
                ? "bg-success"
                : todo.status === "in_progress"
                  ? "bg-info"
                  : "bg-muted-foreground/50",
            )}
          />
          <span
            className={cn(
              "text-[11px] leading-relaxed",
              todo.status === "completed" && "text-muted-foreground line-through",
            )}
          >
            {todo.title}
          </span>
        </div>
      ))}
    </div>
  );
}
