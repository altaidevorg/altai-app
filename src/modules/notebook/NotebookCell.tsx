import { useCallback, useMemo, useState } from "react";
import type { Cell } from "./lib/ipynbParser";
import { renderOutputs } from "./lib/mimeRenderer";
import { cn } from "@/lib/utils";

type NotebookCellProps = {
  cell: Cell;
  index: number;
  isExecuting: boolean;
  onSourceChange: (index: number, source: string) => void;
  onExecute: (index: number) => void;
  onDelete: (index: number) => void;
  onMoveUp: (index: number) => void;
  onMoveDown: (index: number) => void;
  onToggleType: (index: number) => void;
  isFirst: boolean;
  isLast: boolean;
};

export function NotebookCell({
  cell,
  index,
  isExecuting,
  onSourceChange,
  onExecute,
  onDelete,
  onMoveUp,
  onMoveDown,
  onToggleType,
  isFirst,
  isLast,
}: NotebookCellProps) {
  const [focused, setFocused] = useState(false);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
      if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
        e.preventDefault();
        onExecute(index);
      }
    },
    [index, onExecute],
  );

  const executionLabel = cell.executionCount != null ? `[${cell.executionCount}]` : "[ ]";
  const executionAriaLabel = isExecuting
    ? "executing"
    : cell.executionCount != null
      ? `executed ${cell.executionCount}`
      : "not executed";

  const outputs = useMemo(
    () => renderOutputs(cell.outputs, index),
    [cell.outputs, index],
  );

  return (
    <div
      className={cn(
        "group relative border-l-2 transition-colors",
        focused ? "border-blue-500" : "border-transparent hover:border-border",
      )}
    >
      {/* Cell toolbar */}
      <div className="absolute -top-3 right-2 z-10 hidden gap-1 group-hover:flex group-focus-within:flex">
        <CellButton onClick={() => onExecute(index)} title="Run (Ctrl+Enter)" ariaLabel="Run cell (Ctrl+Enter)">
          {isExecuting ? "..." : "\u25B6"}
        </CellButton>
        <CellButton onClick={() => onMoveUp(index)} disabled={isFirst} title="Move up" ariaLabel="Move cell up">
          \u2191
        </CellButton>
        <CellButton onClick={() => onMoveDown(index)} disabled={isLast} title="Move down" ariaLabel="Move cell down">
          \u2193
        </CellButton>
        <CellButton onClick={() => onToggleType(index)} title="Toggle code/markdown" ariaLabel="Toggle code or markdown">
          {cell.cellType === "code" ? "M" : "<>"}
        </CellButton>
        <CellButton onClick={() => onDelete(index)} title="Delete cell" ariaLabel="Delete cell">
          \u00D7
        </CellButton>
      </div>

      <div className="flex">
        {/* Execution count gutter */}
        <div className="w-12 shrink-0 pt-2 text-right pr-2 text-xs text-muted-foreground font-mono select-none">
          {cell.cellType === "code" ? (
            <>
              <span className="sr-only">{executionAriaLabel}</span>
              <span aria-hidden="true">{executionLabel}</span>
            </>
          ) : (
            ""
          )}
        </div>

        <div className="min-w-0 flex-1">
          {/* Source area */}
          {cell.cellType === "markdown" && !focused ? (
            <div
              className="px-3 py-2 text-sm prose prose-sm dark:prose-invert max-w-none cursor-text"
              role="button"
              tabIndex={0}
              aria-label="Edit markdown cell"
              onClick={() => setFocused(true)}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  setFocused(true);
                }
              }}
              dangerouslySetInnerHTML={{ __html: cell.source }}
            />
          ) : (
            <textarea
              className={cn(
                "w-full resize-none bg-muted/30 px-3 py-2 font-mono text-xs",
                "border-0 outline-none focus:ring-1 focus:ring-blue-500/40 rounded",
                cell.cellType === "code" ? "min-h-[2.5rem]" : "min-h-[2rem]",
              )}
              aria-label="Cell source"
              value={cell.source}
              onChange={(e) => onSourceChange(index, e.target.value)}
              onFocus={() => setFocused(true)}
              onBlur={() => setFocused(false)}
              onKeyDown={handleKeyDown}
              rows={Math.max(1, cell.source.split("\n").length)}
              spellCheck={false}
            />
          )}

          {/* Outputs */}
          {outputs.length > 0 && (
            <div className="border-t border-border/40" role="status" aria-live="polite">
              {outputs}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

function CellButton({
  children,
  onClick,
  disabled,
  title,
  ariaLabel,
}: {
  children: React.ReactNode;
  onClick: () => void;
  disabled?: boolean;
  title: string;
  ariaLabel: string;
}) {
  return (
    <button
      type="button"
      className={cn(
        "rounded px-1.5 py-0.5 text-xs bg-muted hover:bg-muted-foreground/20 transition-colors",
        disabled && "opacity-30 pointer-events-none",
      )}
      onClick={onClick}
      disabled={disabled}
      title={title}
      aria-label={ariaLabel}
    >
      {children}
    </button>
  );
}
