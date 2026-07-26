import { Button } from "@/components/ui/button";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetFooter,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { Spinner } from "@/components/ui/spinner";
import { cn } from "@/lib/utils";
import { useChatStore } from "@/modules/ai/store/chatStore";
import type {
  Assignment,
  AssignmentStatus,
} from "@/modules/github/lib/assignments";
import { useAssignmentsStore } from "@/modules/github/store/assignmentsStore";
import {
  GithubIcon,
  GitPullRequestIcon,
  RecordIcon,
  Robot01Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { ReactNode } from "react";

export type BoardCardDetail = {
  title: string;
  source: "issue" | "pr" | "todo" | "draft";
  status: string;
  statusLabel: string;
  number?: number | null;
  url?: string | null;
  body?: string | null;
  meta?: string | null;
};

const ASSIGNMENT_LABEL: Record<AssignmentStatus, string> = {
  dispatching: "Starting",
  running: "In progress",
  "awaiting-approval": "Awaiting approval",
  done: "Ready for review",
  failed: "Failed",
  cancelled: "Cancelled",
};

export function BoardCardDetailsSheet({
  open,
  onOpenChange,
  card,
  assignment,
  statusOptions,
  onStatusChange,
  statusBusy = false,
  onAssign,
  assignControl,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  card: BoardCardDetail | null;
  assignment?: Assignment;
  statusOptions?: { id: string; label: string }[];
  onStatusChange?: (id: string) => void;
  statusBusy?: boolean;
  onAssign?: () => void;
  assignControl?: ReactNode;
}) {
  const switchSession = useChatStore((state) => state.switchSession);
  const cancel = useAssignmentsStore((state) => state.cancel);
  const publishDraftPullRequest = useAssignmentsStore(
    (state) => state.publishDraftPullRequest,
  );
  const applyLocalChanges = useAssignmentsStore(
    (state) => state.applyLocalChanges,
  );

  if (!card) return null;
  const delivery = assignment?.delivery;
  const canPublish =
    assignment?.status === "done" &&
    assignment.source.kind === "issue" &&
    delivery &&
    delivery.status !== "draft-pr";
  const canApplyLocal =
    assignment?.status === "done" &&
    assignment.source.kind === "todo" &&
    assignment.origin === "orchestrator" &&
    delivery &&
    delivery.status !== "applied";

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent className="w-[min(92vw,30rem)] sm:max-w-[30rem]">
        <SheetHeader className="border-b border-border/60 p-5 pr-14">
          <div className="flex items-center gap-2 text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
            <SourceIcon source={card.source} />
            <span>{card.source}</span>
            {card.number ? <span className="font-mono">#{card.number}</span> : null}
          </div>
          <SheetTitle className="text-[16px] leading-snug">
            {card.title}
          </SheetTitle>
          <SheetDescription className="flex items-center gap-2 text-[11.5px]">
            <span className="rounded-full bg-foreground/8 px-2 py-0.5 font-medium text-foreground/75">
              {card.statusLabel}
            </span>
            {card.meta ? <span>{card.meta}</span> : null}
          </SheetDescription>
        </SheetHeader>

        <div className="min-h-0 flex-1 overflow-y-auto p-5">
          {statusOptions && onStatusChange ? (
            <section className="mb-5">
              <label
                htmlFor="board-card-status"
                className="text-[10.5px] font-semibold uppercase tracking-wide text-muted-foreground/70"
              >
                Status
              </label>
              <div className="relative mt-1.5">
                <select
                  id="board-card-status"
                  value={card.status}
                  onChange={(event) => onStatusChange(event.target.value)}
                  disabled={statusBusy}
                  className="h-8 w-full rounded-lg border border-border/60 bg-background px-2.5 text-[12px] text-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:opacity-60"
                >
                  {statusOptions.map((option) => (
                    <option key={option.id} value={option.id}>
                      {option.label}
                    </option>
                  ))}
                </select>
                {statusBusy ? (
                  <Spinner className="absolute right-2 top-2 size-4" />
                ) : null}
              </div>
            </section>
          ) : null}

          <section>
            <h3 className="text-[10.5px] font-semibold uppercase tracking-wide text-muted-foreground/70">
              Description
            </h3>
            {card.body?.trim() ? (
              <p className="mt-2 whitespace-pre-wrap text-[12px] leading-relaxed text-foreground/85">
                {card.body}
              </p>
            ) : (
              <p className="mt-2 text-[12px] text-muted-foreground/55">
                No description provided.
              </p>
            )}
          </section>

          <section className="mt-6">
            <h3 className="text-[10.5px] font-semibold uppercase tracking-wide text-muted-foreground/70">
              Agent
            </h3>
            {assignment ? (
              <div className="mt-2 rounded-xl border border-border/60 bg-card/40 p-3">
                <div className="flex items-center gap-2">
                  <HugeiconsIcon
                    icon={Robot01Icon}
                    size={14}
                    strokeWidth={1.8}
                    className="text-emerald-500"
                  />
                  <span className="text-[12px] font-medium">
                    {ASSIGNMENT_LABEL[assignment.status]}
                  </span>
                  <span
                    className={cn(
                      "ml-auto size-2 rounded-full",
                      assignment.status === "failed"
                        ? "bg-red-500"
                        : assignment.status === "done"
                          ? "bg-violet-500"
                          : "bg-emerald-500",
                    )}
                  />
                </div>
                <dl className="mt-2 grid grid-cols-[5rem_1fr] gap-x-2 gap-y-1 text-[10.5px]">
                  {assignment.runConfig?.agentId ? (
                    <>
                      <dt className="text-muted-foreground">Agent</dt>
                      <dd className="truncate">{assignment.runConfig.agentId}</dd>
                    </>
                  ) : null}
                  {assignment.runConfig?.modelId ? (
                    <>
                      <dt className="text-muted-foreground">Model</dt>
                      <dd className="truncate">{assignment.runConfig.modelId}</dd>
                    </>
                  ) : null}
                  {assignment.runConfig?.branchName ? (
                    <>
                      <dt className="text-muted-foreground">Branch</dt>
                      <dd className="truncate font-mono">
                        {assignment.runConfig.branchName}
                      </dd>
                    </>
                  ) : null}
                </dl>
                <div className="mt-3 flex flex-wrap gap-1.5">
                  <Button
                    size="xs"
                    variant="outline"
                    className="h-7 text-[10.5px]"
                    onClick={() => switchSession(assignment.sessionId)}
                  >
                    Open transcript
                  </Button>
                  {assignment.status === "running" ||
                  assignment.status === "dispatching" ||
                  assignment.status === "awaiting-approval" ? (
                    <Button
                      size="xs"
                      variant="ghost"
                      className="h-7 text-[10.5px] text-red-500"
                      onClick={() => void cancel(assignment.id)}
                    >
                      Cancel
                    </Button>
                  ) : null}
                  {canPublish ? (
                    <Button
                      size="xs"
                      className="h-7 text-[10.5px]"
                      onClick={() => void publishDraftPullRequest(assignment.id)}
                      disabled={delivery.status === "publishing"}
                    >
                      {delivery.status === "publishing" ? (
                        <Spinner className="size-3" />
                      ) : null}
                      {delivery.status === "failed"
                        ? "Retry draft PR"
                        : "Create draft PR"}
                    </Button>
                  ) : null}
                  {canApplyLocal ? (
                    <Button
                      size="xs"
                      className="h-7 text-[10.5px]"
                      onClick={() => void applyLocalChanges(assignment.id)}
                      disabled={delivery.status === "applying"}
                    >
                      {delivery.status === "applying" ? (
                        <Spinner className="size-3" />
                      ) : null}
                      {delivery.status === "failed"
                        ? "Retry apply"
                        : "Apply to workspace"}
                    </Button>
                  ) : null}
                  {assignment.source.kind === "todo" &&
                  delivery?.status === "applied" ? (
                    <span className="inline-flex h-7 items-center rounded-md bg-emerald-500/10 px-2 text-[10.5px] font-medium text-emerald-500">
                      Applied to workspace
                    </span>
                  ) : null}
                  {delivery?.status === "draft-pr" ? (
                    <Button
                      size="xs"
                      className="h-7 text-[10.5px]"
                      onClick={() => void openUrl(delivery.pullUrl)}
                    >
                      Open PR #{delivery.pullNumber}
                    </Button>
                  ) : null}
                </div>
              </div>
            ) : assignControl ? (
              <div className="mt-2">{assignControl}</div>
            ) : onAssign ? (
              <Button
                size="sm"
                variant="outline"
                className="mt-2 h-8 gap-1.5 text-[11.5px]"
                onClick={onAssign}
              >
                <HugeiconsIcon icon={Robot01Icon} size={13} strokeWidth={1.8} />
                Assign agent
              </Button>
            ) : (
              <p className="mt-2 text-[12px] text-muted-foreground/55">
                Draft project notes must be converted to an issue before assignment.
              </p>
            )}
          </section>
        </div>

        {card.url ? (
          <SheetFooter className="border-t border-border/60 p-4">
            <Button
              variant="outline"
              className="h-8 gap-1.5 text-[11.5px]"
              onClick={() => void openUrl(card.url!)}
            >
              <HugeiconsIcon icon={GithubIcon} size={13} strokeWidth={1.8} />
              Open on GitHub
            </Button>
          </SheetFooter>
        ) : null}
      </SheetContent>
    </Sheet>
  );
}

function SourceIcon({ source }: { source: BoardCardDetail["source"] }) {
  const icon =
    source === "pr"
      ? GitPullRequestIcon
      : source === "issue"
        ? RecordIcon
        : Robot01Icon;
  return <HugeiconsIcon icon={icon} size={12} strokeWidth={1.8} />;
}
