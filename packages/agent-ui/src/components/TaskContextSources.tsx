import {
  Attachment02Icon,
  CodeIcon,
  File01Icon,
  TerminalIcon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { ContextSourceToggle } from "./ContextSourceToggle.js";
import { SurfaceSectionHeader } from "./AuxiliarySurface.js";

export type TaskContextSourcesProps = {
  files: string[];
  onAddActiveFile: () => void;
  onChooseFiles: () => void;
  onRemoveFile: (path: string) => void;
  activeFileDisabled: boolean;
  activeFileSelected: boolean;
  includeTerminal: boolean;
  onIncludeTerminalChange: (checked: boolean) => void;
  terminalDetail: string;
  terminalDisabled: boolean;
  includeDiff: boolean;
  onIncludeDiffChange: (checked: boolean) => void;
  diffDetail: string;
  diffDisabled: boolean;
};

/** Basename helper for context file chips. */
export function contextFileName(path: string): string {
  const parts = path.split(/[\\/]/);
  return parts[parts.length - 1] || path;
}

/**
 * Create-task context section: file chips plus terminal / git toggles.
 * Host owns file picking (native dialogs) and live context availability.
 */
export function TaskContextSources({
  files,
  onAddActiveFile,
  onChooseFiles,
  onRemoveFile,
  activeFileDisabled,
  activeFileSelected,
  includeTerminal,
  onIncludeTerminalChange,
  terminalDetail,
  terminalDisabled,
  includeDiff,
  onIncludeDiffChange,
  diffDetail,
  diffDisabled,
}: TaskContextSourcesProps) {
  const count = files.length + Number(includeTerminal) + Number(includeDiff);

  return (
    <section className="altai-task-context-sources border-t border-border-subtle px-3.5 py-3.5">
      <SurfaceSectionHeader
        title="Context"
        description="Add only the evidence this run needs. Sources are snapshotted when work starts."
        count={count}
      />
      <div className="mt-3 overflow-hidden rounded-lg border border-border bg-card">
        <div className="border-b border-border-subtle p-2.5">
          <div className="flex min-w-0 flex-wrap items-center gap-2">
            <span className="inline-flex size-7 shrink-0 items-center justify-center rounded-md bg-muted text-muted-foreground">
              <HugeiconsIcon icon={File01Icon} size={13} strokeWidth={1.75} />
            </span>
            <div className="min-w-28 flex-1">
              <div className="text-[10.5px] font-medium text-foreground">Files</div>
              <div className="truncate text-[9px] text-muted-foreground">
                Add the exact files the agent should read first
              </div>
            </div>
            <div className="ml-auto flex shrink-0 items-center gap-1">
              <button
                type="button"
                onClick={onAddActiveFile}
                disabled={activeFileDisabled}
                className="h-6 rounded-md px-2 text-[9.5px] font-medium text-muted-foreground hover:bg-accent hover:text-foreground disabled:cursor-not-allowed disabled:opacity-40"
              >
                {activeFileSelected ? "Active added" : "Active file"}
              </button>
              <button
                type="button"
                onClick={onChooseFiles}
                className="inline-flex h-6 items-center gap-1 rounded-md border border-border bg-muted px-2 text-[9.5px] font-medium text-foreground hover:bg-accent"
              >
                <HugeiconsIcon
                  icon={Attachment02Icon}
                  size={10}
                  strokeWidth={1.8}
                />
                Choose files
              </button>
            </div>
          </div>
          {files.length ? (
            <div className="mt-2 flex flex-wrap gap-1">
              {files.map((path) => {
                const label = contextFileName(path);
                return (
                  <span
                    key={path}
                    title={path}
                    className="group inline-flex h-6 max-w-full items-center gap-1 rounded-md border border-border bg-muted/55 pl-2 pr-1 text-[9.5px] text-foreground"
                  >
                    <span className="max-w-44 truncate">{label}</span>
                    <button
                      type="button"
                      onClick={() => onRemoveFile(path)}
                      aria-label={`Remove ${label}`}
                      className="inline-flex size-4 items-center justify-center rounded text-muted-foreground hover:bg-foreground/[0.08] hover:text-foreground"
                    >
                      ×
                    </button>
                  </span>
                );
              })}
            </div>
          ) : null}
        </div>
        <ContextSourceToggle
          icon={TerminalIcon}
          label="Terminal output"
          detail={terminalDetail}
          checked={includeTerminal}
          disabled={terminalDisabled}
          onChange={onIncludeTerminalChange}
        />
        <ContextSourceToggle
          icon={CodeIcon}
          label="Working tree changes"
          detail={diffDetail}
          checked={includeDiff}
          disabled={diffDisabled}
          onChange={onIncludeDiffChange}
          className="border-t border-border-subtle"
        />
      </div>
    </section>
  );
}
