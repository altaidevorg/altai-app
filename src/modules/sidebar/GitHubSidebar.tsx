import { Button } from "@/components/ui/button";
import { Spinner } from "@/components/ui/spinner";
import { cn } from "@/lib/utils";
import { useGitHubStore } from "@/modules/github";
import type { ItemKind } from "@/modules/github/lib/items";
import { listItems, type GHItem } from "@/modules/github/lib/items";
import { useRepoSlug } from "@/modules/github/lib/useRepoSlug";
import { openSettingsWindow } from "@/modules/settings/openSettingsWindow";
import {
  SourceControlPanel,
  type SourceControlSummary,
} from "@/modules/source-control";
import {
  ArrowRight01Icon,
  GithubIcon,
  Refresh01Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useEffect, useState } from "react";

type Props = {
  repoRoot: string | null;
  sourceControl: SourceControlSummary;
  onOpenItems: (kind: ItemKind, number?: number) => void;
  onOpenGitGraph: () => void;
  onOpenDiff: (input: {
    path: string;
    repoRoot: string;
    mode: "+" | "-";
    originalPath: string | null;
    title?: string;
  }) => void;
  onBranchSwitched: () => void;
};

export function GitHubSidebar({
  repoRoot,
  sourceControl,
  onOpenItems,
  onOpenGitGraph,
  onOpenDiff,
  onBranchSwitched,
}: Props) {
  return (
    <div className="flex h-full min-h-0 flex-col">
      {repoRoot ? (
        <GitHubRemoteNavigation
          repoRoot={repoRoot}
          onOpenItems={onOpenItems}
        />
      ) : (
        <div className="border-b border-border/50 px-3 py-3">
          <div className="flex items-center gap-2 text-[12px] font-semibold">
            <HugeiconsIcon icon={GithubIcon} size={15} strokeWidth={1.8} />
            GitHub
          </div>
          <p className="mt-1 text-[10.5px] leading-relaxed text-muted-foreground">
            Open a Git repository to load pull requests, issues, and repository
            actions.
          </p>
        </div>
      )}
      <div className="min-h-0 flex-1">
        <SourceControlPanel
          open
          sourceControl={sourceControl}
          onOpenDiff={onOpenDiff}
          onOpenGitGraph={onOpenGitGraph}
          onBranchSwitched={onBranchSwitched}
        />
      </div>
    </div>
  );
}

function GitHubRemoteNavigation({
  repoRoot,
  onOpenItems,
}: {
  repoRoot: string;
  onOpenItems: (kind: ItemKind, number?: number) => void;
}) {
  const connection = useGitHubStore((state) => state.connection);
  const hydrated = useGitHubStore((state) => state.hydrated);
  const refreshConnection = useGitHubStore((state) => state.refresh);
  const slugState = useRepoSlug(repoRoot);
  const [issues, setIssues] = useState<GHItem[]>([]);
  const [pulls, setPulls] = useState<GHItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [reloadKey, setReloadKey] = useState(0);

  const slug = slugState.status === "ready" ? slugState.slug : null;

  useEffect(() => {
    if (!hydrated) void refreshConnection();
  }, [hydrated, refreshConnection]);

  useEffect(() => {
    if (!connection || !slug) {
      setIssues([]);
      setPulls([]);
      setLoading(false);
      setError(null);
      return;
    }
    let alive = true;
    setLoading(true);
    setError(null);
    Promise.all([
      listItems(slug, "issues", "open"),
      listItems(slug, "pulls", "open"),
    ])
      .then(([nextIssues, nextPulls]) => {
        if (!alive) return;
        setIssues(nextIssues);
        setPulls(nextPulls);
      })
      .catch((cause: unknown) => {
        if (!alive) return;
        setError(cause instanceof Error ? cause.message : String(cause));
      })
      .finally(() => {
        if (alive) setLoading(false);
      });
    return () => {
      alive = false;
    };
  }, [connection, reloadKey, slug]);

  const workspaceName =
    repoRoot.split(/[\\/]/).filter(Boolean).pop() ?? "Repository";
  const repositoryLabel = slug ? `${slug.owner}/${slug.repo}` : workspaceName;
  const recentItems = [
    ...pulls.slice(0, 2).map((item) => ({ ...item, kind: "pulls" as const })),
    ...issues.slice(0, 2).map((item) => ({ ...item, kind: "issues" as const })),
  ]
    .sort(
      (left, right) =>
        new Date(right.updated_at).getTime() -
        new Date(left.updated_at).getTime(),
    )
    .slice(0, 3);

  return (
    <section
      aria-label="GitHub repository"
      className="shrink-0 border-b border-border/50 bg-card/60"
    >
      <div className="flex items-center gap-2 px-3 pb-2 pt-3">
        <HugeiconsIcon
          icon={GithubIcon}
          size={15}
          strokeWidth={1.8}
          className="shrink-0 text-muted-foreground"
        />
        <span className="min-w-0 flex-1 truncate text-[12px] font-semibold">
          {repositoryLabel}
        </span>
        <button
          type="button"
          aria-label="Refresh GitHub items"
          title="Refresh GitHub items"
          disabled={loading || !connection || !slug}
          onClick={() => setReloadKey((key) => key + 1)}
          className="flex size-6 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-muted hover:text-foreground disabled:cursor-not-allowed disabled:opacity-40"
        >
          {loading ? (
            <Spinner className="size-3.5" />
          ) : (
            <HugeiconsIcon icon={Refresh01Icon} size={13} strokeWidth={1.9} />
          )}
        </button>
      </div>

      {!hydrated || slugState.status === "loading" ? (
        <div className="flex items-center gap-2 px-3 pb-3 text-[10.5px] text-muted-foreground">
          <Spinner className="size-3" />
          Loading GitHub…
        </div>
      ) : !connection ? (
        <div className="px-3 pb-3">
          <p className="mb-2 text-[10.5px] leading-relaxed text-muted-foreground">
            Connect GitHub to browse remote pull requests and issues. Local Git
            actions remain available below.
          </p>
          <Button
            size="xs"
            variant="outline"
            className="h-7 text-[10.5px]"
            onClick={() => openSettingsWindow("github")}
          >
            Connect GitHub
          </Button>
        </div>
      ) : !slug ? (
        <p className="px-3 pb-3 text-[10.5px] leading-relaxed text-muted-foreground">
          This repository does not have a GitHub origin.
        </p>
      ) : (
        <>
          <div className="grid grid-cols-2 gap-1.5 px-3 pb-2">
            <RemoteButton
              label="Pull requests"
              count={pulls.length}
              disabled={loading}
              onClick={() => onOpenItems("pulls")}
            />
            <RemoteButton
              label="Issues"
              count={issues.length}
              disabled={loading}
              onClick={() => onOpenItems("issues")}
            />
          </div>
          {error ? (
            <p
              role="alert"
              className="mx-3 mb-2 line-clamp-2 rounded-md bg-destructive/10 px-2 py-1.5 text-[10px] text-destructive"
              title={error}
            >
              {error}
            </p>
          ) : recentItems.length > 0 ? (
            <div className="border-t border-border/35 pb-1 pt-1">
              <p className="px-3 py-1 text-[9.5px] font-semibold uppercase tracking-wide text-muted-foreground/60">
                Recently updated
              </p>
              {recentItems.map((item) => (
                <button
                  key={`${item.kind}-${item.number}`}
                  type="button"
                  onClick={() => onOpenItems(item.kind, item.number)}
                  className="group flex w-full items-center gap-2 px-3 py-1.5 text-left transition-colors hover:bg-foreground/[0.04]"
                >
                  <span
                    className={cn(
                      "shrink-0 rounded px-1 py-0.5 text-[9px] font-semibold",
                      item.kind === "pulls"
                        ? "bg-violet-500/10 text-violet-500"
                        : "bg-sky-500/10 text-sky-500",
                    )}
                  >
                    {item.kind === "pulls" ? "PR" : "Issue"} #{item.number}
                  </span>
                  <span className="min-w-0 flex-1 truncate text-[10.5px] text-foreground/85">
                    {item.title}
                  </span>
                  <HugeiconsIcon
                    icon={ArrowRight01Icon}
                    size={11}
                    strokeWidth={2}
                    className="shrink-0 text-muted-foreground/40 transition-transform group-hover:translate-x-0.5"
                  />
                </button>
              ))}
            </div>
          ) : null}
        </>
      )}
    </section>
  );
}

function RemoteButton({
  label,
  count,
  disabled,
  onClick,
}: {
  label: string;
  count: number;
  disabled: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={onClick}
      className="flex h-8 items-center gap-2 rounded-lg border border-border/55 bg-background/45 px-2 text-[10.5px] font-medium text-foreground transition-colors hover:border-border hover:bg-muted/60 disabled:cursor-wait disabled:opacity-50"
    >
      <span className="truncate">{label}</span>
      <span className="ml-auto rounded-full bg-foreground/[0.07] px-1.5 text-[9.5px] tabular-nums text-muted-foreground">
        {count}
      </span>
    </button>
  );
}
