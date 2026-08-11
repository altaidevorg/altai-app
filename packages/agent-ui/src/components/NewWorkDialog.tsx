import { useState } from "react";
import { cn } from "../lib/cn.js";

export type NewWorkDialogProps = {
  open: boolean;
  projectLabel: string;
  onClose: () => void;
  onCreate: (input: {
    title: string;
    description: string;
    acceptanceCriteria: string;
  }) => void;
  className?: string;
};

/**
 * New Work dialog (SCREENS.md) — title, project (display), description, criteria.
 */
export function NewWorkDialog({
  open,
  projectLabel,
  onClose,
  onCreate,
  className,
}: NewWorkDialogProps) {
  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");
  const [acceptanceCriteria, setAcceptanceCriteria] = useState("");

  if (!open) return null;

  const canCreate = title.trim().length > 0;

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label="New Work"
      className={cn(
        "altai-new-work-dialog fixed inset-0 z-50 flex items-center justify-center bg-background/70 p-4",
        className,
      )}
    >
      <div className="w-full max-w-md rounded-lg border border-border bg-card p-4 shadow-sm">
        <h2 className="text-[13px] font-semibold text-foreground">New Work</h2>
        <p className="mt-1 text-[11px] text-muted-foreground">
          Project: {projectLabel}
        </p>
        <label className="mt-3 block text-[10.5px] font-medium text-muted-foreground">
          Title
          <input
            value={title}
            onChange={(event) => setTitle(event.target.value)}
            className="mt-1 w-full rounded-md border border-border bg-background px-2.5 py-1.5 text-[12px] text-foreground outline-none focus:border-ring"
            autoFocus
          />
        </label>
        <label className="mt-3 block text-[10.5px] font-medium text-muted-foreground">
          Description
          <textarea
            value={description}
            onChange={(event) => setDescription(event.target.value)}
            rows={3}
            className="mt-1 w-full resize-y rounded-md border border-border bg-background px-2.5 py-1.5 text-[12px] text-foreground outline-none focus:border-ring"
          />
        </label>
        <label className="mt-3 block text-[10.5px] font-medium text-muted-foreground">
          Acceptance criteria
          <textarea
            value={acceptanceCriteria}
            onChange={(event) => setAcceptanceCriteria(event.target.value)}
            rows={3}
            className="mt-1 w-full resize-y rounded-md border border-border bg-background px-2.5 py-1.5 text-[12px] text-foreground outline-none focus:border-ring"
          />
        </label>
        <div className="mt-4 flex justify-end gap-2">
          <button
            type="button"
            onClick={onClose}
            className="inline-flex h-7 items-center rounded-md px-2.5 text-[11px] text-muted-foreground hover:bg-muted hover:text-foreground"
          >
            Cancel
          </button>
          <button
            type="button"
            disabled={!canCreate}
            onClick={() => {
              onCreate({
                title: title.trim(),
                description: description.trim(),
                acceptanceCriteria: acceptanceCriteria.trim(),
              });
              setTitle("");
              setDescription("");
              setAcceptanceCriteria("");
            }}
            className="inline-flex h-7 items-center rounded-md bg-foreground px-2.5 text-[11px] font-medium text-background disabled:cursor-not-allowed disabled:opacity-40"
          >
            Create
          </button>
        </div>
      </div>
    </div>
  );
}
