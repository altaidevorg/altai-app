import {
  Cancel01Icon,
  Delete02Icon,
  PencilEdit02Icon,
  Tick02Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import type { RefObject } from "react";
import { cn } from "../lib/cn.js";
import { RowIconButton } from "./RowIconButton.js";

export type SessionRowProps = {
  title: string;
  snippet?: string;
  active: boolean;
  renaming: boolean;
  renameValue: string;
  onPick: () => void;
  onStartRename: () => void;
  onCommitRename: () => void;
  onCancelRename: () => void;
  onRenameValueChange: (value: string) => void;
  onDelete: () => void;
  renameInputRef: RefObject<HTMLInputElement | null>;
};

/**
 * Chat session row for the history list. Supports inline rename and delete
 * actions. Purely presentational; the host owns session data and mutation
 * handlers.
 */
export function SessionRow({
  title,
  snippet,
  active,
  renaming,
  renameValue,
  onPick,
  onStartRename,
  onCommitRename,
  onCancelRename,
  onRenameValueChange,
  onDelete,
  renameInputRef,
}: SessionRowProps) {
  const displayTitle = title || "New chat";

  return (
    <div
      role="button"
      tabIndex={renaming ? -1 : 0}
      onClick={() => {
        if (renaming) return;
        onPick();
      }}
      onKeyDown={(e) => {
        if (renaming) return;
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          onPick();
        }
      }}
      className={cn(
        "group relative flex cursor-pointer items-start gap-2 rounded-md px-2 py-1.5 text-left transition-colors",
        active ? "bg-accent text-foreground" : "hover:bg-accent/50",
      )}
    >
      <div className="flex min-w-0 flex-1 flex-col gap-0.5">
        {renaming ? (
          <input
            ref={renameInputRef}
            value={renameValue}
            onChange={(e) => onRenameValueChange(e.target.value)}
            onClick={(e) => e.stopPropagation()}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                onCommitRename();
              } else if (e.key === "Escape") {
                e.preventDefault();
                onCancelRename();
              }
              e.stopPropagation();
            }}
            onBlur={onCommitRename}
            className="w-full bg-transparent text-[12px] font-medium text-foreground outline-none"
          />
        ) : (
          <span
            className={cn(
              "truncate text-[12px] font-medium",
              active ? "text-foreground" : "text-foreground/90",
            )}
          >
            {displayTitle}
          </span>
        )}
        {snippet ? (
          <span className="line-clamp-1 text-[10.5px] leading-snug text-muted-foreground">
            {snippet}
          </span>
        ) : null}
      </div>

      {renaming ? (
        <div className="flex shrink-0 items-center gap-0.5">
          <RowIconButton
            title="Save"
            onClick={(e) => {
              e.stopPropagation();
              onCommitRename();
            }}
          >
            <HugeiconsIcon icon={Tick02Icon} size={11} strokeWidth={2} />
          </RowIconButton>
          <RowIconButton
            title="Cancel"
            onClick={(e) => {
              e.stopPropagation();
              onCancelRename();
            }}
          >
            <HugeiconsIcon icon={Cancel01Icon} size={10} strokeWidth={2} />
          </RowIconButton>
        </div>
      ) : (
        <div className="flex shrink-0 items-center gap-0.5 opacity-0 transition-opacity group-hover:opacity-100">
          <RowIconButton
            title="Rename"
            onClick={(e) => {
              e.stopPropagation();
              onStartRename();
            }}
          >
            <HugeiconsIcon icon={PencilEdit02Icon} size={11} strokeWidth={1.75} />
          </RowIconButton>
          <RowIconButton
            title="Delete"
            onClick={(e) => {
              e.stopPropagation();
              onDelete();
            }}
            tone="destructive"
          >
            <HugeiconsIcon icon={Delete02Icon} size={11} strokeWidth={1.75} />
          </RowIconButton>
        </div>
      )}
    </div>
  );
}
