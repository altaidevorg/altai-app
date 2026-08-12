import { InspectorEmpty } from "./InspectorEmpty.js";

export type AgentsInspectorTask = {
  taskId: string;
  displayName?: string | null;
  agentName?: string | null;
  childChatId: string;
};

export type AgentsInspectorProps = {
  tasks: AgentsInspectorTask[];
};

/**
 * Active subagent tasks as a flat list.
 */
export function AgentsInspector({ tasks }: AgentsInspectorProps) {
  if (!tasks.length) {
    return (
      <InspectorEmpty>
        Delegated research, review, and test tasks will stay visible here.
      </InspectorEmpty>
    );
  }
  return (
    <ul className="divide-y divide-border-subtle">
      {tasks.map((task) => (
        <li key={task.taskId} className="flex gap-2 py-2">
          <span className="mt-1.5 size-1.5 shrink-0 animate-pulse rounded-full bg-foreground/70" />
          <div className="min-w-0 flex-1">
            <div className="truncate text-[11px] font-medium text-foreground">
              {task.displayName ?? task.agentName ?? "Subagent"}
            </div>
            <div className="mt-0.5 truncate font-mono text-[10.5px] text-muted-foreground">
              {task.childChatId}
            </div>
          </div>
        </li>
      ))}
    </ul>
  );
}
