import { InspectorEmpty } from "./InspectorEmpty.js";

export type ResearchInspectorEvent = {
  id: string;
  label: string;
  detail?: string;
  createdAt: number;
};

export type ResearchInspectorProps = {
  events: ResearchInspectorEvent[];
};

/**
 * Run-inspector panel for research activity (web searches, page fetches).
 * Purely presentational; the host supplies the filtered event list.
 */
export function ResearchInspector({ events }: ResearchInspectorProps) {
  if (!events.length) {
    return (
      <InspectorEmpty>
        Web searches, fetched pages, and paper lookups will appear here.
      </InspectorEmpty>
    );
  }
  return (
    <div className="space-y-2">
      {[...events].reverse().map((item) => (
        <div
          key={item.id}
          className="rounded-md border border-border bg-muted/30 px-2.5 py-2"
        >
          <div className="flex items-center gap-2">
            <span className="size-1.5 shrink-0 rounded-full bg-info" />
            <span className="min-w-0 flex-1 truncate text-[11px] font-medium">
              {item.label}
            </span>
            <time
              className="text-[9px] tabular-nums text-muted-foreground"
              dateTime={new Date(item.createdAt).toISOString()}
            >
              {new Date(item.createdAt).toLocaleTimeString([], {
                hour: "2-digit",
                minute: "2-digit",
              })}
            </time>
          </div>
          {item.detail ? (
            <div className="mt-1 pl-3.5 text-[10px] text-muted-foreground">
              {item.detail}
            </div>
          ) : null}
        </div>
      ))}
    </div>
  );
}
