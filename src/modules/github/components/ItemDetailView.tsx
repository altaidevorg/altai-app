import { MarkdownCode } from "@/components/ai-elements/markdown-code";
import { Button } from "@/components/ui/button";
import { Spinner } from "@/components/ui/spinner";
import { Textarea } from "@/components/ui/textarea";
import { cn } from "@/lib/utils";
import {
  addComment,
  getIssue,
  getPull,
  type GHCheckRun,
  type GHComment,
  type GHItem,
  type GHPullDetail,
  type GHReview,
  type ItemKind,
  listCheckRuns,
  listComments,
  listPullReviews,
  mergePull,
  relativeTime,
  type RepoSlug,
  setIssueState,
} from "@/modules/github/lib/items";
import {
  Alert02Icon,
  ArrowLeft01Icon,
  ArrowReloadHorizontalIcon,
  CheckmarkCircle01Icon,
  CheckmarkCircle02Icon,
  Clock01Icon,
  GitMergeIcon,
  SentIcon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Streamdown } from "streamdown";
import { AssignAgentButton } from "./AssignAgentButton";
import {
  Avatar,
  ItemStateIcon,
  itemState,
  Labels,
  StateBadge,
} from "./itemBits";

type Props = {
  slug: RepoSlug;
  kind: ItemKind;
  number: number;
  onBack: () => void;
  onMutated: () => void;
};

const MD = { code: MarkdownCode };

export function ItemDetailView({
  slug,
  kind,
  number,
  onBack,
  onMutated,
}: Props) {
  const [item, setItem] = useState<GHItem | GHPullDetail | null>(null);
  const [comments, setComments] = useState<GHComment[]>([]);
  const [reviews, setReviews] = useState<GHReview[]>([]);
  const [checks, setChecks] = useState<GHCheckRun[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [busy, setBusy] = useState<
    null | "state" | "merge" | "comment"
  >(null);
  const [draft, setDraft] = useState("");
  const requestRef = useRef(0);

  const load = useCallback(async () => {
    const requestId = ++requestRef.current;
    setLoading(true);
    setError(null);
    setNotice(null);
    try {
      const detail =
        kind === "pulls"
          ? await getPull(slug, number)
          : await getIssue(slug, number);
      if (requestId !== requestRef.current) return;
      setItem(detail);
      const pullDetail =
        kind === "pulls" ? (detail as GHPullDetail) : null;

      const results = await Promise.allSettled([
        listComments(slug, number),
        kind === "pulls"
          ? listPullReviews(slug, number)
          : Promise.resolve([]),
        pullDetail
          ? listCheckRuns(slug, pullDetail.head.sha)
          : Promise.resolve([]),
      ]);
      if (requestId !== requestRef.current) return;

      const [commentResult, reviewResult, checkResult] = results;
      setComments(
        commentResult.status === "fulfilled" ? commentResult.value : [],
      );
      setReviews(
        reviewResult.status === "fulfilled" ? reviewResult.value : [],
      );
      setChecks(checkResult.status === "fulfilled" ? checkResult.value : []);

      const failedParts: string[] = [];
      if (commentResult.status === "rejected") failedParts.push("comments");
      if (reviewResult.status === "rejected") failedParts.push("reviews");
      if (checkResult.status === "rejected") failedParts.push("checks");
      if (failedParts.length > 0) {
        setNotice(
          `Loaded the item, but ${failedParts.join(", ")} could not be refreshed.`,
        );
      }
    } catch (cause) {
      if (requestId === requestRef.current) {
        setItem(null);
        setError(cause instanceof Error ? cause.message : String(cause));
      }
    } finally {
      if (requestId === requestRef.current) setLoading(false);
    }
  }, [slug, kind, number]);

  useEffect(() => {
    void load();
    return () => {
      requestRef.current += 1;
    };
  }, [load]);

  const latestReviewState = useMemo(() => {
    const byUser = new Map<string, GHReview>();
    for (const review of reviews) {
      const login = review.user?.login;
      if (!login || review.state === "COMMENTED") continue;
      byUser.set(login, review);
    }
    return [...byUser.values()];
  }, [reviews]);

  const approvals = latestReviewState.filter(
    (review) => review.state === "APPROVED",
  );
  const changeRequests = latestReviewState.filter(
    (review) => review.state === "CHANGES_REQUESTED",
  );
  const failedChecks = checks.filter(
    (check) =>
      check.status === "completed" &&
      !["success", "neutral", "skipped"].includes(check.conclusion ?? ""),
  );
  const pendingChecks = checks.filter(
    (check) => check.status !== "completed",
  );
  const passedChecks = checks.filter(
    (check) =>
      check.status === "completed" &&
      ["success", "neutral", "skipped"].includes(check.conclusion ?? ""),
  );

  const toggleState = async () => {
    if (!item || busy) return;
    setBusy("state");
    setError(null);
    try {
      await setIssueState(
        slug,
        number,
        item.state === "open" ? "closed" : "open",
      );
      await load();
      onMutated();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(null);
    }
  };

  const doMerge = async () => {
    if (busy) return;
    setBusy("merge");
    setError(null);
    try {
      const result = await mergePull(slug, number);
      if (!result.merged) {
        throw new Error(result.message || "GitHub did not merge this pull request.");
      }
      await load();
      onMutated();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(null);
    }
  };

  const submitComment = async () => {
    const body = draft.trim();
    if (!body || busy) return;
    setBusy("comment");
    setError(null);
    try {
      const comment = await addComment(slug, number, body);
      setComments((current) => [...current, comment]);
      setDraft("");
      onMutated();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(null);
    }
  };

  const pull = item && "merged" in item ? (item as GHPullDetail) : null;
  const canMerge =
    !!pull &&
    pull.state === "open" &&
    !pull.draft &&
    !pull.merged &&
    pull.mergeable === true &&
    changeRequests.length === 0 &&
    failedChecks.length === 0 &&
    pendingChecks.length === 0;
  const state = item ? itemState(item) : "open";
  const resolution =
    item && state === "merged"
      ? `merged ${relativeTime(
          item.merged_at ??
            item.pull_request?.merged_at ??
            item.updated_at,
        )}`
      : item && (state === "closed" || state === "not_planned")
        ? `${state === "not_planned" ? "closed as not planned" : "closed"} ${relativeTime(item.closed_at ?? item.updated_at)}`
        : null;

  return (
    <div className="mx-auto flex h-full w-full max-w-5xl flex-col px-4 py-3">
      <div className="mb-2 flex items-center justify-between gap-2">
        <button
          type="button"
          onClick={onBack}
          className="-ml-1.5 flex w-fit items-center gap-1 rounded-md px-1.5 py-1 text-[11px] text-muted-foreground transition-colors hover:bg-muted/50 hover:text-foreground"
        >
          <HugeiconsIcon icon={ArrowLeft01Icon} size={13} strokeWidth={2} />
          Back to {kind === "pulls" ? "pull requests" : "issues"}
        </button>
        <div className="flex items-center gap-1">
          <Button
            size="xs"
            variant="ghost"
            className="size-7 p-0"
            aria-label="Refresh item"
            title="Refresh"
            onClick={() => void load()}
            disabled={loading}
          >
            {loading ? (
              <Spinner className="size-3" />
            ) : (
              <HugeiconsIcon
                icon={ArrowReloadHorizontalIcon}
                size={12}
                strokeWidth={1.9}
              />
            )}
          </Button>
          <Button
            size="xs"
            variant="ghost"
            className="h-7 text-[10px]"
            onClick={() => item && void openUrl(item.html_url)}
            disabled={!item}
          >
            Open on GitHub ↗
          </Button>
        </div>
      </div>

      {loading && !item ? (
        <DetailSkeleton />
      ) : !item ? (
        <div className="flex flex-1 flex-col items-center justify-center gap-2 text-center">
          <span className="flex size-10 items-center justify-center rounded-xl bg-destructive/10 text-destructive">
            <HugeiconsIcon icon={Alert02Icon} size={18} strokeWidth={1.8} />
          </span>
          <p className="text-[12px] font-medium text-foreground">
            This GitHub item could not be loaded.
          </p>
          <p className="max-w-sm text-[10.5px] text-destructive">
            {error ?? "Not found."}
          </p>
          <Button
            size="xs"
            variant="outline"
            className="mt-1 h-7 text-[10px]"
            onClick={() => void load()}
          >
            Retry
          </Button>
        </div>
      ) : (
        <div className="min-h-0 flex-1 overflow-y-auto pb-3">
          <header className="rounded-2xl border border-border/50 bg-card/40 p-3">
            <div className="flex items-start gap-2.5">
              <span className="mt-0.5">
                <ItemStateIcon state={state} kind={kind} size={19} />
              </span>
              <div className="min-w-0 flex-1">
                <div className="flex flex-wrap items-start gap-2">
                  <h2 className="min-w-0 flex-1 text-[15px] font-semibold leading-snug text-foreground">
                    {item.title}{" "}
                    <span className="font-normal text-muted-foreground/55">
                      #{item.number}
                    </span>
                  </h2>
                  <StateBadge state={state} kind={kind} />
                </div>
                <div className="mt-1.5 flex flex-wrap items-center gap-1.5 text-[10px] text-muted-foreground">
                  <Avatar url={item.user?.avatar_url} size={16} />
                  <span className="font-medium text-foreground/80">
                    {item.user?.login ?? "unknown"}
                  </span>
                  <span>opened {relativeTime(item.created_at)}</span>
                  {resolution ? <span>· {resolution}</span> : null}
                  {pull ? (
                    <span className="rounded-md bg-muted/55 px-1.5 py-0.5 font-mono text-[9.5px]">
                      {pull.head.ref} → {pull.base.ref}
                    </span>
                  ) : null}
                </div>
                <div className="mt-2">
                  <Labels labels={item.labels} />
                </div>
              </div>
            </div>

            <div className="mt-3 flex flex-wrap items-center gap-1.5 border-t border-border/40 pt-3">
              {canMerge ? (
                <Button
                  size="xs"
                  className="h-7 gap-1.5 text-[10.5px]"
                  onClick={() => void doMerge()}
                  disabled={!!busy}
                >
                  {busy === "merge" ? (
                    <Spinner className="size-3" />
                  ) : (
                    <HugeiconsIcon
                      icon={GitMergeIcon}
                      size={12}
                      strokeWidth={1.9}
                    />
                  )}
                  Merge pull request
                </Button>
              ) : null}
              <Button
                size="xs"
                variant="outline"
                className="h-7 gap-1.5 text-[10px]"
                onClick={() => void toggleState()}
                disabled={!!busy}
              >
                {busy === "state" ? (
                  <Spinner className="size-3" />
                ) : (
                  <HugeiconsIcon
                    icon={CheckmarkCircle02Icon}
                    size={12}
                    strokeWidth={1.9}
                  />
                )}
                {item.state === "open"
                  ? kind === "pulls"
                    ? "Close pull request"
                    : "Close issue"
                  : "Reopen"}
              </Button>
              <div className="ml-auto">
                <AssignAgentButton
                  kind={kind === "pulls" ? "pr" : "issue"}
                  slug={slug}
                  number={number}
                  title={item.title}
                  body={item.body}
                  url={item.html_url}
                  variant="button"
                />
              </div>
            </div>
          </header>

          {notice ? (
            <p className="mt-2 rounded-lg border border-amber-500/20 bg-amber-500/[0.06] px-2.5 py-1.5 text-[9.5px] text-amber-600 dark:text-amber-400">
              {notice}
            </p>
          ) : null}
          {error ? (
            <p
              role="alert"
              className="mt-2 rounded-lg border border-destructive/20 bg-destructive/[0.06] px-2.5 py-1.5 text-[10px] text-destructive"
            >
              {error}
            </p>
          ) : null}

          <div
            className={cn(
              "mt-3 grid gap-3",
              pull && "lg:grid-cols-[minmax(0,1fr)_17rem]",
            )}
          >
            <main className="min-w-0 space-y-3">
              <section className="rounded-xl border border-border/50 bg-card/30 px-3 py-3">
                <p className="mb-2 text-[9px] font-semibold uppercase tracking-wide text-muted-foreground/55">
                  Description
                </p>
                {item.body?.trim() ? (
                  <Streamdown
                    className="prose-sm max-w-none [&>*:first-child]:mt-0 [&>*:last-child]:mb-0"
                    components={MD}
                  >
                    {item.body}
                  </Streamdown>
                ) : (
                  <p className="text-[10.5px] italic text-muted-foreground/60">
                    No description provided.
                  </p>
                )}
              </section>

              <section>
                <div className="mb-1.5 flex items-center justify-between px-1">
                  <p className="text-[9px] font-semibold uppercase tracking-wide text-muted-foreground/55">
                    Discussion
                  </p>
                  <span className="text-[9px] text-muted-foreground/50">
                    {comments.length} comment
                    {comments.length === 1 ? "" : "s"}
                  </span>
                </div>
                <div className="space-y-2">
                  {comments.map((comment) => (
                    <CommentCard key={comment.id} comment={comment} />
                  ))}
                  {comments.length === 0 ? (
                    <p className="rounded-xl border border-dashed border-border/50 px-3 py-4 text-center text-[10px] text-muted-foreground/55">
                      No discussion yet.
                    </p>
                  ) : null}
                </div>
              </section>

              <section className="rounded-xl border border-border/50 bg-card/30 p-2 transition-colors focus-within:border-ring/50 focus-within:ring-1 focus-within:ring-ring/20">
                <Textarea
                  value={draft}
                  onChange={(event) => setDraft(event.target.value)}
                  placeholder="Add context, feedback, or a decision…"
                  aria-label="New comment"
                  rows={3}
                  className="resize-y border-0 bg-transparent px-1 text-[11px] shadow-none focus-visible:ring-0"
                />
                <div className="flex items-center justify-between gap-2 border-t border-border/40 pt-2">
                  <span className="pl-1 text-[9px] text-muted-foreground/50">
                    Markdown supported
                  </span>
                  <Button
                    size="xs"
                    className="h-7 gap-1.5 text-[10px]"
                    onClick={() => void submitComment()}
                    disabled={!draft.trim() || !!busy}
                  >
                    {busy === "comment" ? (
                      <Spinner className="size-3" />
                    ) : (
                      <HugeiconsIcon
                        icon={SentIcon}
                        size={12}
                        strokeWidth={1.9}
                      />
                    )}
                    Comment
                  </Button>
                </div>
              </section>
            </main>

            {pull ? (
              <PullReadiness
                pull={pull}
                approvals={approvals}
                changeRequests={changeRequests}
                checks={checks}
                failedChecks={failedChecks}
                pendingChecks={pendingChecks}
                passedChecks={passedChecks}
              />
            ) : null}
          </div>
        </div>
      )}
    </div>
  );
}

function PullReadiness({
  pull,
  approvals,
  changeRequests,
  checks,
  failedChecks,
  pendingChecks,
  passedChecks,
}: {
  pull: GHPullDetail;
  approvals: GHReview[];
  changeRequests: GHReview[];
  checks: GHCheckRun[];
  failedChecks: GHCheckRun[];
  pendingChecks: GHCheckRun[];
  passedChecks: GHCheckRun[];
}) {
  const readiness =
    pull.draft
      ? { label: "Draft", detail: "Mark ready on GitHub before merging.", tone: "neutral" as const }
      : pull.mergeable === false
        ? { label: "Conflicts", detail: "Resolve branch conflicts first.", tone: "error" as const }
        : changeRequests.length > 0
          ? { label: "Changes requested", detail: "Review feedback is blocking merge.", tone: "error" as const }
          : failedChecks.length > 0
            ? { label: "Checks failed", detail: "Fix failing validation before merge.", tone: "error" as const }
            : pendingChecks.length > 0
              ? { label: "Checks running", detail: "Waiting for automated validation.", tone: "warning" as const }
              : pull.mergeable === null
                ? { label: "Calculating", detail: "GitHub is computing mergeability.", tone: "warning" as const }
                : { label: "Ready to merge", detail: "No detected merge blockers.", tone: "success" as const };

  return (
    <aside className="space-y-2">
      <section className="rounded-xl border border-border/50 bg-card/35 p-3">
        <p className="text-[9px] font-semibold uppercase tracking-wide text-muted-foreground/55">
          Merge readiness
        </p>
        <div className="mt-2 flex items-start gap-2">
          <ReadinessIcon tone={readiness.tone} />
          <div>
            <p className="text-[10.5px] font-semibold text-foreground">
              {readiness.label}
            </p>
            <p className="mt-0.5 text-[9px] leading-relaxed text-muted-foreground/65">
              {readiness.detail}
            </p>
          </div>
        </div>
      </section>

      <section className="rounded-xl border border-border/50 bg-card/35 p-3">
        <p className="text-[9px] font-semibold uppercase tracking-wide text-muted-foreground/55">
          Change summary
        </p>
        <dl className="mt-2 grid grid-cols-2 gap-1.5">
          <SummaryValue label="Commits" value={pull.commits ?? "—"} />
          <SummaryValue label="Files" value={pull.changed_files ?? "—"} />
          <SummaryValue
            label="Additions"
            value={pull.additions ?? "—"}
            tone="success"
          />
          <SummaryValue
            label="Deletions"
            value={pull.deletions ?? "—"}
            tone="error"
          />
        </dl>
      </section>

      <section className="rounded-xl border border-border/50 bg-card/35 p-3">
        <div className="flex items-center justify-between">
          <p className="text-[9px] font-semibold uppercase tracking-wide text-muted-foreground/55">
            Reviews
          </p>
          <span className="text-[9px] text-muted-foreground/50">
            {approvals.length} approved
          </span>
        </div>
        <div className="mt-2 space-y-1.5">
          {changeRequests.map((review) => (
            <PersonState
              key={review.id}
              review={review}
              label="Changes requested"
              tone="error"
            />
          ))}
          {approvals.map((review) => (
            <PersonState
              key={review.id}
              review={review}
              label="Approved"
              tone="success"
            />
          ))}
          {(pull.requested_reviewers ?? []).map((user) => (
            <div key={user.login} className="flex items-center gap-2">
              <Avatar url={user.avatar_url} size={15} />
              <span className="min-w-0 flex-1 truncate text-[9.5px]">
                {user.login}
              </span>
              <span className="text-[8.5px] text-muted-foreground/55">
                Requested
              </span>
            </div>
          ))}
          {approvals.length === 0 &&
          changeRequests.length === 0 &&
          (pull.requested_reviewers?.length ?? 0) === 0 ? (
            <p className="text-[9px] text-muted-foreground/55">
              No review activity.
            </p>
          ) : null}
        </div>
      </section>

      <section className="rounded-xl border border-border/50 bg-card/35 p-3">
        <div className="flex items-center justify-between">
          <p className="text-[9px] font-semibold uppercase tracking-wide text-muted-foreground/55">
            Checks
          </p>
          <span className="text-[9px] text-muted-foreground/50">
            {passedChecks.length}/{checks.length} passed
          </span>
        </div>
        <div className="mt-2 space-y-1">
          {checks.slice(0, 8).map((check) => (
            <button
              key={check.id}
              type="button"
              disabled={!check.html_url}
              onClick={() => check.html_url && void openUrl(check.html_url)}
              className="flex w-full items-center gap-1.5 rounded-md py-0.5 text-left disabled:cursor-default"
            >
              <CheckIcon check={check} />
              <span className="min-w-0 flex-1 truncate text-[9px] text-foreground/85">
                {check.name}
              </span>
            </button>
          ))}
          {checks.length === 0 ? (
            <p className="text-[9px] text-muted-foreground/55">
              No checks reported for this commit.
            </p>
          ) : null}
        </div>
      </section>
    </aside>
  );
}

function ReadinessIcon({
  tone,
}: {
  tone: "neutral" | "error" | "warning" | "success";
}) {
  const icon =
    tone === "error"
      ? Alert02Icon
      : tone === "warning"
        ? Clock01Icon
        : CheckmarkCircle01Icon;
  return (
    <span
      className={cn(
        "flex size-7 shrink-0 items-center justify-center rounded-lg",
        tone === "error"
          ? "bg-red-500/10 text-red-500"
          : tone === "warning"
            ? "bg-amber-500/10 text-amber-500"
            : tone === "success"
              ? "bg-emerald-500/10 text-emerald-500"
              : "bg-muted text-muted-foreground",
      )}
    >
      <HugeiconsIcon icon={icon} size={14} strokeWidth={1.9} />
    </span>
  );
}

function SummaryValue({
  label,
  value,
  tone = "neutral",
}: {
  label: string;
  value: string | number;
  tone?: "neutral" | "success" | "error";
}) {
  return (
    <div className="rounded-lg bg-muted/40 px-2 py-1.5">
      <dt className="text-[8px] text-muted-foreground/55">{label}</dt>
      <dd
        className={cn(
          "text-[10.5px] font-semibold",
          tone === "success"
            ? "text-emerald-500"
            : tone === "error"
              ? "text-red-500"
              : "text-foreground",
        )}
      >
        {tone === "success" && value !== "—" ? "+" : ""}
        {tone === "error" && value !== "—" ? "−" : ""}
        {value}
      </dd>
    </div>
  );
}

function PersonState({
  review,
  label,
  tone,
}: {
  review: GHReview;
  label: string;
  tone: "success" | "error";
}) {
  return (
    <div className="flex items-center gap-2">
      <Avatar url={review.user?.avatar_url} size={15} />
      <span className="min-w-0 flex-1 truncate text-[9.5px]">
        {review.user?.login ?? "unknown"}
      </span>
      <span
        className={cn(
          "text-[8.5px] font-medium",
          tone === "success" ? "text-emerald-500" : "text-red-500",
        )}
      >
        {label}
      </span>
    </div>
  );
}

function CheckIcon({ check }: { check: GHCheckRun }) {
  const failed =
    check.status === "completed" &&
    !["success", "neutral", "skipped"].includes(check.conclusion ?? "");
  const pending = check.status !== "completed";
  return (
    <HugeiconsIcon
      icon={
        failed
          ? Alert02Icon
          : pending
            ? Clock01Icon
            : CheckmarkCircle01Icon
      }
      size={11}
      strokeWidth={1.9}
      className={
        failed
          ? "text-red-500"
          : pending
            ? "text-amber-500"
            : "text-emerald-500"
      }
    />
  );
}

function CommentCard({ comment }: { comment: GHComment }) {
  return (
    <article className="rounded-xl border border-border/50 bg-card/30 px-3 py-2.5">
      <div className="mb-1.5 flex items-center gap-2 text-[9.5px] text-muted-foreground">
        <Avatar url={comment.user?.avatar_url} size={16} />
        <span className="font-medium text-foreground/80">
          {comment.user?.login ?? "unknown"}
        </span>
        <span>· {relativeTime(comment.created_at)}</span>
      </div>
      <Streamdown
        className="prose-sm max-w-none [&>*:first-child]:mt-0 [&>*:last-child]:mb-0"
        components={MD}
      >
        {comment.body}
      </Streamdown>
    </article>
  );
}

function DetailSkeleton() {
  return (
    <div className="space-y-3" aria-hidden>
      <div className="h-28 animate-pulse rounded-2xl bg-muted/60" />
      <div className="grid gap-3 lg:grid-cols-[minmax(0,1fr)_17rem]">
        <div className="h-64 animate-pulse rounded-xl bg-muted/50" />
        <div className="h-64 animate-pulse rounded-xl bg-muted/40" />
      </div>
    </div>
  );
}
