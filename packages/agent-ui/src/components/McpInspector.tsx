import { cn } from "../lib/cn.js";
import { InspectorEmpty } from "./InspectorEmpty.js";

export type McpInspectorEvent = {
  id: string;
  label: string;
  detail?: string;
  tone?: "default" | "success" | "warning" | "error";
  createdAt: number;
};

export type McpInspectorProps = {
  events: McpInspectorEvent[];
};

/**
 * MCP call activity as a flat list.
 */
export function McpInspector({ events }: McpInspectorProps) {
  if (!events.length) {
    return (
      <InspectorEmpty>
        MCP server calls will appear here when the agent uses a connected tool.
      </InspectorEmpty>
    );
  }
  return (
    <ul className="divide-y divide-border-subtle">
      {[...events].reverse().map((item) => (
        <li key={item.id} className="flex gap-2 py-2">
          <span
            className={cn(
              "mt-1.5 size-1.5 shrink-0 rounded-full",
              item.tone === "error"
                ? "bg-destructive"
                : item.tone === "success"
                  ? "bg-foreground"
                  : "bg-muted-foreground/50",
            )}
          />
          <div className="min-w-0 flex-1">
            <div className="flex items-baseline gap-2">
              <span className="min-w-0 flex-1 truncate text-[11px] font-medium text-foreground">
                {item.label}
              </span>
              <time
                className="shrink-0 text-[10.5px] tabular-nums text-muted-foreground"
                dateTime={new Date(item.createdAt).toISOString()}
              >
                {new Date(item.createdAt).toLocaleTimeString([], {
                  hour: "2-digit",
                  minute: "2-digit",
                })}
              </time>
            </div>
            {item.detail ? (
              <div className="mt-0.5 text-[10.5px] text-muted-foreground">
                {item.detail}
              </div>
            ) : null}
          </div>
        </li>
      ))}
    </ul>
  );
}
