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
 * Queued plan edits as a flat list with one primary review action.
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
    <div>
      <div className="mb-1.5 flex items-center justify-between gap-2 px-0.5">
        <p className="text-[11px] text-muted-foreground">
          {queue.length} proposed change{queue.length === 1 ? "" : "s"}
        </p>
        <button
          type="button"
          onClick={onOpenReview}
          className="inline-flex h-7 items-center rounded-md bg-foreground px-2 text-[11px] font-medium text-background transition-opacity hover:opacity-90"
        >
          Open change review
        </button>
      </div>
      <ul className="divide-y divide-border-subtle">
        {queue.map((change) => {
          const beforeLines = change.originalContent.split("\n").length;
          const afterLines = change.proposedContent.split("\n").length;
          const delta = afterLines - beforeLines;
          const name = basename(change.path);
          return (
            <li key={change.id} className="flex items-center gap-2 py-2">
              <HugeiconsIcon
                icon={FileEditIcon}
                size={12}
                strokeWidth={1.75}
                className="shrink-0 text-muted-foreground"
              />
              <div className="min-w-0 flex-1">
                <div className="truncate font-mono text-[11px] font-medium text-foreground">
                  {name}
                </div>
                <div className="truncate font-mono text-[10.5px] text-muted-foreground">
                  {change.path}
                </div>
              </div>
              {change.isNewFile ? (
                <span className="text-[10.5px] text-muted-foreground">new</span>
              ) : (
                <span
                  className={cn(
                    "text-[10.5px] tabular-nums",
                    delta >= 0 ? "text-foreground" : "text-muted-foreground",
                  )}
                >
                  {delta >= 0 ? "+" : ""}
                  {delta}L
                </span>
              )}
            </li>
          );
        })}
      </ul>
    </div>
  );
}
