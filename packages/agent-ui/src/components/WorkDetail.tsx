import type { ReactNode } from "react";
import { cn } from "../lib/cn.js";
import { SurfaceEmptyState } from "./AuxiliarySurface.js";
import { SurfaceLoadingState } from "./SurfaceLoadingState.js";
import { SurfaceInlineError } from "./SurfaceInlineError.js";

export type WorkDetailPrimaryAction =
  | "ready"
  | "start"
  | "open_run"
  | "accept"
  | "return"
  | "reopen";

export type WorkDetailAttemptRow = {
  id: string;
  label: string;
  phaseLabel: string;
  onOpenRun?: () => void;
};

export type WorkDetailProps = {
  status: "loading" | "ready" | "error" | "not_found";
  title?: string;
  stateLabel?: string;
  /** Inbox condition on this Work — a distinct axis from the state, so it
   *  renders as its own chip, never folded into `stateLabel`. */
  attentionLabel?: string | null;
  projectLabel?: string;
  updatedLabel?: string;
  description?: string;
  acceptanceCriteria?: string;
  blocker?: string | null;
  sourceLabel?: string | null;
  onOpenSource?: () => void;
  primaryActions?: WorkDetailPrimaryAction[];
  onPrimaryAction?: (action: WorkDetailPrimaryAction) => void;
  onBack?: () => void;
  onEdit?: () => void;
  onCopyId?: () => void;
  onCancelWork?: () => void;
  attempts?: WorkDetailAttemptRow[];
  history?: { id: string; label: string }[];
  errorMessage?: string;
  onRetry?: () => void;
  className?: string;
};

const PRIMARY_LABEL: Record<WorkDetailPrimaryAction, string> = {
  ready: "Ready",
  start: "Start",
  open_run: "Open run",
  accept: "Accept",
  return: "Return",
  reopen: "Reopen",
};

/**
 * Work OS detail screen (SCREENS.md). Single scroll; sticky Accept/Return via
 * primaryActions when in review.
 */
export function WorkDetail({
  status,
  title,
  stateLabel,
  attentionLabel,
  projectLabel,
  updatedLabel,
  description,
  acceptanceCriteria,
  blocker,
  sourceLabel,
  onOpenSource,
  primaryActions = [],
  onPrimaryAction,
  onBack,
  onEdit,
  onCopyId,
  onCancelWork,
  attempts = [],
  history = [],
  errorMessage,
  onRetry,
  className,
}: WorkDetailProps) {
  if (status === "loading") {
    return <SurfaceLoadingState className={className}>Loading Work…</SurfaceLoadingState>;
  }
  if (status === "error") {
    return (
      <SurfaceInlineError
        className={cn("m-3", className)}
        message={errorMessage ?? "Work failed to load."}
        onDismiss={onRetry}
      />
    );
  }
  if (status === "not_found") {
    return (
      <SurfaceEmptyState
        className={cn("border-0 bg-transparent", className)}
        title="Work not found"
        description="It may have been removed or is in another project."
        action={
          onBack ? (
            <button
              type="button"
              onClick={onBack}
              className="inline-flex h-7 items-center rounded-md border border-border px-2.5 text-[11px] font-medium"
            >
              Back to Work
            </button>
          ) : undefined
        }
      />
    );
  }

  return (
    <div
      className={cn(
        "altai-work-detail flex h-full min-h-0 flex-col overflow-hidden bg-card",
        className,
      )}
    >
      <header className="flex shrink-0 flex-col gap-2 border-b border-border-subtle px-3 py-2">
        <div className="flex items-start gap-2">
          {onBack ? (
            <button
              type="button"
              onClick={onBack}
              aria-label="Back to Work list"
              className="mt-0.5 inline-flex size-7 shrink-0 items-center justify-center rounded-md text-muted-foreground hover:bg-muted hover:text-foreground"
            >
              ←
            </button>
          ) : null}
          <div className="min-w-0 flex-1">
            <h2 className="truncate text-[13px] font-semibold text-foreground">
              {title}
            </h2>
            <p className="mt-0.5 flex flex-wrap items-center gap-x-2 gap-y-0.5 text-[10.5px] text-muted-foreground">
              {stateLabel ? <span>{stateLabel}</span> : null}
              {attentionLabel ? (
                <span className="rounded-full bg-warning/15 px-1.5 py-px text-[9.5px] font-medium text-warning">
                  {attentionLabel}
                </span>
              ) : null}
              {projectLabel ? <span>{projectLabel}</span> : null}
              {updatedLabel ? <span>{updatedLabel}</span> : null}
            </p>
          </div>
          <div className="flex shrink-0 items-center gap-1">
            {primaryActions.map((action) => (
              <button
                key={action}
                type="button"
                onClick={() => onPrimaryAction?.(action)}
                className={cn(
                  "inline-flex h-7 items-center rounded-md px-2.5 text-[11px] font-medium",
                  action === "accept"
                    ? "bg-foreground text-background"
                    : action === "return"
                      ? "border border-border text-foreground hover:bg-muted"
                      : "bg-foreground text-background hover:opacity-90",
                )}
              >
                {PRIMARY_LABEL[action]}
              </button>
            ))}
            <OverflowMenu onEdit={onEdit} onCopyId={onCopyId} onCancelWork={onCancelWork} />
          </div>
        </div>
      </header>

      <div className="min-h-0 flex-1 space-y-4 overflow-y-auto px-3 py-3">
        <Section title="Description">
          <p className="whitespace-pre-wrap text-[12px] text-foreground/90">
            {description?.trim() ? description : "No description."}
          </p>
        </Section>
        <Section title="Acceptance criteria">
          <p className="whitespace-pre-wrap text-[12px] text-foreground/90">
            {acceptanceCriteria?.trim()
              ? acceptanceCriteria
              : "No acceptance criteria."}
          </p>
        </Section>
        {blocker ? (
          <Section title="Blocker">
            <p className="text-[12px] text-destructive">{blocker}</p>
          </Section>
        ) : null}
        {sourceLabel ? (
          <Section title="Source">
            {onOpenSource ? (
              <button
                type="button"
                onClick={onOpenSource}
                className="text-[12px] text-foreground underline-offset-2 hover:underline"
              >
                {sourceLabel}
              </button>
            ) : (
              <p className="text-[12px] text-muted-foreground">{sourceLabel}</p>
            )}
          </Section>
        ) : null}

        <Section title="Attempts">
          {attempts.length === 0 ? (
            <p className="text-[12px] text-muted-foreground">No attempts yet.</p>
          ) : (
            <ul className="divide-y divide-border-subtle overflow-hidden rounded-lg border border-border">
              {attempts.map((attempt) => (
                <li
                  key={attempt.id}
                  className="flex items-center gap-2 px-2.5 py-2 text-[11px]"
                >
                  <span className="min-w-0 flex-1 truncate font-medium text-foreground">
                    {attempt.label}
                  </span>
                  <span className="text-muted-foreground">{attempt.phaseLabel}</span>
                  {attempt.onOpenRun ? (
                    <button
                      type="button"
                      onClick={attempt.onOpenRun}
                      className="rounded-md px-2 py-1 text-foreground hover:bg-muted"
                    >
                      Open run
                    </button>
                  ) : null}
                </li>
              ))}
            </ul>
          )}
        </Section>

        {history.length > 0 ? (
          <Section title="History">
            <ul className="space-y-1">
              {history.map((event) => (
                <li
                  key={event.id}
                  className="text-[11px] text-muted-foreground"
                >
                  {event.label}
                </li>
              ))}
            </ul>
          </Section>
        ) : null}
      </div>
    </div>
  );
}

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section>
      <h3 className="mb-1 text-[10.5px] font-semibold uppercase tracking-wide text-muted-foreground">
        {title}
      </h3>
      {children}
    </section>
  );
}

function OverflowMenu({
  onEdit,
  onCopyId,
  onCancelWork,
}: {
  onEdit?: () => void;
  onCopyId?: () => void;
  onCancelWork?: () => void;
}) {
  if (!onEdit && !onCopyId && !onCancelWork) return null;
  return (
    <details className="relative">
      <summary
        aria-label="Work actions"
        className="flex size-7 cursor-pointer list-none items-center justify-center rounded-md text-muted-foreground hover:bg-muted hover:text-foreground"
      >
        ⋯
      </summary>
      <div className="absolute right-0 z-10 mt-1 min-w-36 rounded-md border border-border bg-card py-1 shadow-sm">
        {onEdit ? (
          <button
            type="button"
            onClick={onEdit}
            className="block w-full px-3 py-1.5 text-left text-[11px] hover:bg-muted"
          >
            Edit
          </button>
        ) : null}
        {onCopyId ? (
          <button
            type="button"
            onClick={onCopyId}
            className="block w-full px-3 py-1.5 text-left text-[11px] hover:bg-muted"
          >
            Copy ID
          </button>
        ) : null}
        {onCancelWork ? (
          <button
            type="button"
            onClick={onCancelWork}
            className="block w-full px-3 py-1.5 text-left text-[11px] text-destructive hover:bg-muted"
          >
            Cancel Work
          </button>
        ) : null}
      </div>
    </details>
  );
}
