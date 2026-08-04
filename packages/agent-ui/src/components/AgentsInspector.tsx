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
 * Run-inspector panel listing active subagent tasks. Purely presentational;
 * the host supplies the task list from its agent meta store.
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
    <div className="space-y-2">
      {tasks.map((task) => (
        <div
          key={task.taskId}
          className="rounded-md border border-border bg-muted/30 px-2.5 py-2"
        >
          <div className="flex items-center gap-2">
            <span className="size-1.5 animate-pulse rounded-full bg-info" />
            <span className="truncate text-[11px] font-medium">
              {task.displayName ?? task.agentName ?? "Subagent"}
            </span>
          </div>
          <div className="mt-1 truncate pl-3.5 font-mono text-[9.5px] text-muted-foreground">
            {task.childChatId}
          </div>
        </div>
      ))}
    </div>
  );
}
