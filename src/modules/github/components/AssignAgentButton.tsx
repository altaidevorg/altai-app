import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Spinner } from "@/components/ui/spinner";
import { Textarea } from "@/components/ui/textarea";
import { cn } from "@/lib/utils";
import { useChatStore } from "@/modules/ai/store/chatStore";
import { useAgentsStore } from "@/modules/ai/store/agentsStore";
import type { RepoSlug } from "@/modules/github/lib/items";
import {
  assignGitHubItem,
  isItemAssigned,
  useAssignmentsStore,
} from "@/modules/github/store/assignmentsStore";
import { usePreferencesStore } from "@/modules/settings/preferences";
import { Robot01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useState } from "react";
import {
  AgentRunOptionsFields,
  type AgentRunOptions,
} from "./AgentRunOptionsFields";

type Props = {
  kind: "issue" | "pr";
  slug: RepoSlug;
  number: number;
  title: string;
  body: string | null;
  url: string;
  /** "chip" for list rows, "button" for the detail actions bar. */
  variant?: "chip" | "button";
};

/** Dispatch an agent for a GitHub issue/PR. Shows an assigned state once a run
 *  exists, and surfaces dispatch errors (e.g. no model configured). */
export function AssignAgentButton({
  kind,
  slug,
  number,
  title,
  body,
  url,
  variant = "chip",
}: Props) {
  const assignments = useAssignmentsStore((s) => s.assignments);
  const assigned = isItemAssigned(assignments, kind, number);
  const activeAgentId = useAgentsStore((s) => s.activeId);
  const selectedModelId = useChatStore((s) => s.selectedModelId);
  const defaultPermissionMode = usePreferencesStore((s) => s.permissionMode);
  const bypassEnabled = usePreferencesStore((s) => s.bypassPermissionsEnabled);
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [additionalInstructions, setAdditionalInstructions] = useState("");
  const [isolateWorktree, setIsolateWorktree] = useState(true);
  const [runOptions, setRunOptions] = useState<AgentRunOptions>(() => ({
    agentId: activeAgentId,
    modelId: selectedModelId,
    permissionMode:
      defaultPermissionMode === "bypass" && !bypassEnabled
        ? "ask"
        : defaultPermissionMode,
  }));

  const onClick = async (e: React.MouseEvent) => {
    e.stopPropagation();
    if (busy || assigned) return;
    setError(null);
    setOpen(true);
  };

  const assign = async () => {
    if (busy || assigned) return;
    setBusy(true);
    setError(null);
    try {
      await assignGitHubItem({
        kind,
        slug,
        number,
        title,
        body,
        url,
        runConfig: runOptions,
        additionalInstructions,
        isolate: isolateWorktree,
      });
      setOpen(false);
      setAdditionalInstructions("");
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  if (assigned) {
    return (
      <span
        className={cn(
          "flex shrink-0 items-center gap-1 font-medium text-emerald-500",
          variant === "button" ? "text-[12px]" : "text-[10px]",
        )}
      >
        <HugeiconsIcon icon={Robot01Icon} size={12} strokeWidth={1.9} />
        Assigned
      </span>
    );
  }

  const trigger =
    variant === "button" ? (
      <Button
        size="xs"
        variant="outline"
        className="h-7 gap-1.5 text-[11px]"
        onClick={onClick}
        disabled={busy}
        title={error ?? undefined}
      >
        {busy ? (
          <Spinner className="size-3.5" />
        ) : (
          <HugeiconsIcon icon={Robot01Icon} size={12} strokeWidth={1.9} />
        )}
        Assign agent
      </Button>
    ) : (
      <button
        type="button"
        onClick={onClick}
        disabled={busy}
        title={error ?? "Assign an agent to work on this"}
        className={cn(
          "flex shrink-0 items-center gap-1 rounded-md px-1.5 py-1 text-[10px] font-medium transition-colors",
          error
            ? "text-red-500 hover:bg-red-500/10"
            : "text-muted-foreground/70 hover:bg-muted/60 hover:text-foreground",
        )}
      >
        {busy ? (
          <Spinner className="size-3" />
        ) : (
          <HugeiconsIcon icon={Robot01Icon} size={11} strokeWidth={1.9} />
        )}
        Assign
      </button>
    );

  return (
    <>
      {trigger}
      <Dialog
        open={open}
        onOpenChange={(next) => {
          if (!busy) setOpen(next);
        }}
      >
        <DialogContent
          className="gap-4 sm:max-w-lg"
          onClick={(event) => event.stopPropagation()}
        >
          <DialogHeader>
            <DialogTitle>
              Assign {kind === "pr" ? "pull request" : "issue"} to an agent
            </DialogTitle>
            <DialogDescription className="line-clamp-2">
              {kind === "pr" ? "PR" : "Issue"} #{number} · {title}
            </DialogDescription>
          </DialogHeader>

          <AgentRunOptionsFields
            value={runOptions}
            onChange={setRunOptions}
            disabled={busy}
          />

          <button
            type="button"
            aria-pressed={isolateWorktree}
            onClick={() => setIsolateWorktree((value) => !value)}
            disabled={busy}
            className={cn(
              "flex items-start gap-2 rounded-xl border px-3 py-2 text-left transition-colors disabled:opacity-50",
              isolateWorktree
                ? "border-emerald-500/35 bg-emerald-500/8"
                : "border-amber-500/35 bg-amber-500/8",
            )}
          >
            <span
              className={cn(
                "mt-0.5 flex size-4 shrink-0 items-center justify-center rounded border text-[10px]",
                isolateWorktree
                  ? "border-emerald-500 bg-emerald-500 text-white"
                  : "border-amber-500",
              )}
            >
              {isolateWorktree ? "✓" : ""}
            </span>
            <span>
              <span className="block text-[11.5px] font-medium text-foreground">
                Use an isolated git worktree
              </span>
              <span className="block text-[10.5px] leading-relaxed text-muted-foreground">
                Keeps this run on its own branch and protects your active working
                tree. Recommended for every coding task.
              </span>
            </span>
          </button>

          <label htmlFor={`assignment-instructions-${kind}-${number}`}>
            <span className="text-[10px] font-medium uppercase tracking-wide text-muted-foreground">
              Additional instructions
            </span>
            <Textarea
              id={`assignment-instructions-${kind}-${number}`}
              value={additionalInstructions}
              onChange={(event) => setAdditionalInstructions(event.target.value)}
              placeholder="Testing requirements, files to avoid, implementation notes…"
              rows={4}
              disabled={busy}
              className="mt-1.5 resize-none text-[11.5px]"
            />
          </label>

          {error ? (
            <p role="alert" className="text-[11px] text-destructive">
              {error}
            </p>
          ) : null}

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => setOpen(false)}
              disabled={busy}
            >
              Cancel
            </Button>
            <Button
              type="button"
              onClick={() => void assign()}
              disabled={busy}
              className="gap-1.5"
            >
              {busy ? <Spinner className="size-3.5" /> : null}
              Start background run
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
