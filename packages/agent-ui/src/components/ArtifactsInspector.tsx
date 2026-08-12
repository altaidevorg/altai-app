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
 * Emitted artifact files as a flat list.
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
    <ul className="divide-y divide-border-subtle">
      {[...items].reverse().map((item) => (
        <li key={item.id} className="flex items-center gap-2 py-2">
          <HugeiconsIcon
            icon={FileEditIcon}
            size={12}
            strokeWidth={1.75}
            className="shrink-0 text-muted-foreground"
          />
          <div className="min-w-0 flex-1">
            <div
              className="truncate text-[11px] font-medium text-foreground"
              title={item.path}
            >
              {basename(item.path)}
            </div>
            <div className="mt-0.5 truncate font-mono text-[10.5px] text-muted-foreground">
              {item.path}
            </div>
          </div>
          <button
            type="button"
            onClick={() => onOpenFile(item.path)}
            className="inline-flex h-7 items-center rounded-md px-2 text-[11px] font-medium text-foreground transition-colors hover:bg-foreground/[0.06]"
          >
            Open
          </button>
        </li>
      ))}
    </ul>
  );
}
