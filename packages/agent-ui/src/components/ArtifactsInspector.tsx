import { FileEditIcon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { InspectorEmpty } from "./InspectorEmpty.js";

export type ArtifactsInspectorItem = {
  id: string;
  path: string;
};

export type ArtifactsInspectorProps = {
  items: ArtifactsInspectorItem[];
  onOpenFile: (path: string) => void;
};

function basename(path: string): string {
  return path.split(/[\\/]/).pop() || path;
}

/**
 * Run-inspector panel listing emitted artifact files. Purely presentational;
 * the host supplies items and the open-file handler.
 */
export function ArtifactsInspector({
  items,
  onOpenFile,
}: ArtifactsInspectorProps) {
  if (!items.length) {
    return (
      <InspectorEmpty>
        Files emitted by experiments and execution jobs will appear here.
      </InspectorEmpty>
    );
  }
  return (
    <div className="space-y-2">
      {[...items].reverse().map((item) => (
        <div
          key={item.id}
          className="flex items-center gap-2 rounded-md border border-border bg-muted/30 px-2.5 py-2"
        >
          <HugeiconsIcon
            icon={FileEditIcon}
            size={12}
            strokeWidth={1.75}
            className="shrink-0 text-muted-foreground"
          />
          <div className="min-w-0 flex-1">
            <div
              className="truncate text-[11px] font-medium"
              title={item.path}
            >
              {basename(item.path)}
            </div>
            <div className="mt-0.5 truncate font-mono text-[9.5px] text-muted-foreground">
              {item.path}
            </div>
          </div>
          <button
            type="button"
            onClick={() => onOpenFile(item.path)}
            className="rounded-md bg-foreground/[0.07] px-1.5 py-1 text-[10px] font-medium text-foreground hover:bg-foreground/[0.12]"
          >
            Open
          </button>
        </div>
      ))}
    </div>
  );
}
