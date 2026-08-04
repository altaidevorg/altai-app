import {
  ArrowDown01Icon,
  Cancel01Icon,
  FileEditIcon,
  FilePlusIcon,
  FolderAddIcon,
  Tick02Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useState } from "react";
import { cn } from "../lib/cn.js";
import { UnifiedDiffPreview } from "./UnifiedDiffPreview.js";

export type PlanRowProps = {
  path: string;
  kind: string;
  isNewFile: boolean;
  description?: string;
  originalContent: string;
  proposedContent: string;
  stats: { added: number; removed: number } | null;
  busy: boolean;
  onOpenDiff: () => void;
  onApply: () => void;
  onReject: () => void;
};

function basename(p: string): string {
  const norm = p.replace(/\\/g, "/");
  const idx = norm.lastIndexOf("/");
  return idx === -1 ? p : norm.slice(idx + 1);
}

/**
 * Plan diff review row: shows file path, change stats, inline diff toggle,
 * and apply/reject/open-diff actions. The host computes diff stats and
 * owns the `QueuedEdit` data; this component is purely presentational.
 */
export function PlanRow({
  path,
  kind,
  isNewFile,
  description,
  originalContent,
  proposedContent,
  stats,
  busy,
  onOpenDiff,
  onApply,
  onReject,
}: PlanRowProps) {
  const [open, setOpen] = useState(false);
  const isDir = kind === "create_directory";
  const isNew = isNewFile && !isDir;
  const Icon = isDir
    ? FolderAddIcon
    : isNew
      ? FilePlusIcon
      : FileEditIcon;

  return (
    <li className="group/row overflow-hidden rounded-md border border-border bg-muted/30">
      <div className="flex items-start gap-2 px-2.5 py-1.5">
        <button
          type="button"
          onClick={() => !isDir && setOpen((v) => !v)}
          disabled={isDir}
          className={cn(
            "mt-0.5 shrink-0 text-muted-foreground transition-transform",
            open && "rotate-180",
            isDir && "invisible",
          )}
          aria-label="Toggle diff"
        >
          <HugeiconsIcon icon={ArrowDown01Icon} size={11} strokeWidth={1.75} />
        </button>
        <HugeiconsIcon
          icon={Icon}
          size={13}
          strokeWidth={1.75}
          className="mt-0.5 shrink-0 text-muted-foreground"
        />
        <div className="min-w-0 flex-1">
          <div className="flex items-baseline gap-1.5 font-mono text-[11.5px]">
            <span className="truncate text-foreground">
              {basename(path)}
            </span>
            {isNew ? (
              <span className="text-[10px] text-success">new</span>
            ) : null}
          </div>
          <div className="truncate font-mono text-[10px] text-muted-foreground">
            {path}
          </div>
          {stats ? (
            <div className="mt-0.5 flex items-center gap-2 text-[10px] tabular-nums">
              <span className="text-success">+{stats.added}</span>
              <span className="text-destructive">−{stats.removed}</span>
              <span className="text-muted-foreground">
                {kind === "multi_edit" ? "multi-edit" : kind}
              </span>
            </div>
          ) : (
            <div className="mt-0.5 text-[10px] text-muted-foreground">
              {description ?? "create directory"}
            </div>
          )}
        </div>
        <div className="flex shrink-0 items-center gap-0.5 opacity-0 transition-opacity group-hover/row:opacity-100 group-focus-within/row:opacity-100">
          {!isDir ? (
            <button
              type="button"
              onClick={onOpenDiff}
              disabled={busy}
              aria-label="Open full diff"
              className="inline-flex size-5 items-center justify-center rounded text-muted-foreground hover:bg-foreground/[0.08] hover:text-foreground disabled:opacity-40"
            >
              <HugeiconsIcon icon={FileEditIcon} size={11} strokeWidth={1.75} />
            </button>
          ) : null}
          <button
            type="button"
            onClick={onReject}
            disabled={busy}
            aria-label="Reject"
            className="inline-flex size-5 items-center justify-center rounded text-muted-foreground hover:bg-foreground/[0.08] hover:text-foreground disabled:opacity-40"
          >
            <HugeiconsIcon icon={Cancel01Icon} size={11} strokeWidth={1.75} />
          </button>
          <button
            type="button"
            onClick={onApply}
            disabled={busy}
            aria-label="Apply this change"
            className="inline-flex size-5 items-center justify-center rounded text-success hover:bg-success/10 focus-visible:bg-success/15 disabled:opacity-40"
          >
            <HugeiconsIcon icon={Tick02Icon} size={11} strokeWidth={1.75} />
          </button>
        </div>
      </div>
      {open && !isDir ? (
        <div className="border-t border-border/40 bg-muted/20 px-2.5 py-2">
          <UnifiedDiffPreview
            original={originalContent}
            proposed={proposedContent}
          />
        </div>
      ) : null}
    </li>
  );
}
