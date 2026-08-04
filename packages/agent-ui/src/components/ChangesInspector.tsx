import { FileEditIcon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { cn } from "../lib/cn.js";
import { InspectorEmpty } from "./InspectorEmpty.js";

export type ChangesInspectorItem = {
  id: string;
  path: string;
  originalContent: string;
  proposedContent: string;
  isNewFile: boolean;
};

export type ChangesInspectorProps = {
  queue: ChangesInspectorItem[];
  onOpenReview: () => void;
};

function basename(path: string): string {
  return path.split(/[/\\]/).pop() || path;
}

/**
 * Run-inspector panel summarizing queued plan edits. Purely presentational;
 * the host supplies the queue and the open-review handler.
 */
export function ChangesInspector({
  queue,
  onOpenReview,
}: ChangesInspectorProps) {
  if (!queue.length) {
    return (
      <InspectorEmpty>
        Planned and agent-made changes will appear here for review.
      </InspectorEmpty>
    );
  }
  return (
    <div className="space-y-2">
      <div className="rounded-md border border-border/50 bg-muted/20 p-2.5 text-[11px] leading-relaxed text-foreground">
        <div>
          {queue.length} proposed change{queue.length === 1 ? " is" : "s are"}{" "}
          waiting for review.
        </div>
        <button
          type="button"
          onClick={onOpenReview}
          className="mt-2 rounded-md bg-foreground px-2 py-1 text-[10.5px] font-medium text-background"
        >
          Open change review
        </button>
      </div>
      {queue.map((change) => {
        const beforeLines = change.originalContent.split("\n").length;
        const afterLines = change.proposedContent.split("\n").length;
        const delta = afterLines - beforeLines;
        const name = basename(change.path);
        return (
          <div
            key={change.id}
            className="rounded-md border border-border bg-muted/30 px-2.5 py-2"
          >
            <div className="flex items-center gap-2">
              <HugeiconsIcon
                icon={FileEditIcon}
                size={12}
                strokeWidth={1.75}
                className="shrink-0 text-muted-foreground"
              />
              <span className="min-w-0 flex-1 truncate font-mono text-[10.5px] font-medium">
                {name}
              </span>
              {change.isNewFile ? (
                <span className="text-[9.5px] text-success">new</span>
              ) : null}
              {!change.isNewFile ? (
                <span
                  className={cn(
                    "text-[9.5px] tabular-nums",
                    delta >= 0 ? "text-success" : "text-destructive",
                  )}
                >
                  {delta >= 0 ? "+" : ""}
                  {delta}L
                </span>
              ) : null}
            </div>
            <div className="mt-1 truncate pl-5 font-mono text-[9.5px] text-muted-foreground">
              {change.path}
            </div>
          </div>
        );
      })}
    </div>
  );
}
