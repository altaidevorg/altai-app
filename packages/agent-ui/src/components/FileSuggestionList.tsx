import { useEffect, useRef, type ReactNode } from "react";
import { cn } from "../lib/cn.js";

export type FileSuggestionListProps = {
  files: readonly string[];
  activeIndex: number;
  indexing: boolean;
  truncated: boolean;
  hasWorkspace: boolean;
  onPick: (file: string) => void;
  onHover: (index: number) => void;
  /** Host resolves file-type icons (Desktop: explorer icon theme). */
  iconUrlForFile: (fileName: string) => string;
  /** Optional host spinner for the indexing empty state. */
  indexingIndicator?: ReactNode;
};

/**
 * Workspace file suggestion list for the composer `@` picker.
 * Hosts own popover/portal chrome and icon resolution.
 */
export function FileSuggestionList({
  files,
  activeIndex,
  indexing,
  truncated,
  hasWorkspace,
  onPick,
  onHover,
  iconUrlForFile,
  indexingIndicator,
}: FileSuggestionListProps) {
  const itemRefs = useRef<Array<HTMLButtonElement | null>>([]);

  useEffect(() => {
    const el = itemRefs.current[activeIndex];
    if (!el) return;
    el.scrollIntoView({ block: "nearest" });
  }, [activeIndex]);

  return (
    <div className="w-80 overflow-hidden rounded-lg border border-border/80 bg-popover p-0 text-popover-foreground shadow-xl">
      <div className="border-b border-border/60 px-2.5 py-1.5 text-[10px] font-medium uppercase tracking-wide text-muted-foreground/70">
        Workspace files
      </div>
      {!hasWorkspace ? (
        <div className="px-3 py-3 text-[11px] text-muted-foreground">
          No workspace open
        </div>
      ) : indexing && files.length === 0 ? (
        <div className="flex items-center gap-2 px-3 py-3 text-[11px] text-muted-foreground">
          {indexingIndicator}
          <span>Indexing workspace…</span>
        </div>
      ) : files.length === 0 ? (
        <div className="px-3 py-3 text-[11px] text-muted-foreground">
          No matching files
        </div>
      ) : (
        <>
          <div className="max-h-64 overflow-y-auto py-1">
            {files.map((path, idx) => {
              const slash = path.lastIndexOf("/");
              const name = slash === -1 ? path : path.slice(slash + 1);
              const dir = slash === -1 ? "" : path.slice(0, slash);
              return (
                <button
                  key={path}
                  ref={(el) => {
                    itemRefs.current[idx] = el;
                  }}
                  type="button"
                  onClick={() => onPick(path)}
                  onMouseEnter={() => onHover(idx)}
                  className={cn(
                    "mx-1 my-0.5 flex w-[calc(100%-0.5rem)] items-center gap-2 rounded-md px-2 py-1.5 text-left text-[12px] text-popover-foreground",
                    idx === activeIndex
                      ? "bg-foreground/[0.065]"
                      : "hover:bg-foreground/[0.055]",
                  )}
                >
                  <img
                    src={iconUrlForFile(name)}
                    alt=""
                    className="size-4 shrink-0"
                  />
                  <span className="flex min-w-0 flex-1 items-baseline gap-1.5">
                    <span className="truncate font-medium">{name}</span>
                    {dir ? (
                      <span className="truncate text-[10.5px] text-muted-foreground">
                        {dir}
                      </span>
                    ) : null}
                  </span>
                </button>
              );
            })}
          </div>
          {truncated ? (
            <div className="border-t border-border/60 px-2.5 py-1.5 text-[10px] text-muted-foreground">
              Workspace is large - refine your query to narrow results.
            </div>
          ) : null}
        </>
      )}
    </div>
  );
}
