import { Button } from "@/components/ui/button";
import { Spinner } from "@/components/ui/spinner";
import { useGitHubStore } from "@/modules/github";
import { githubCapabilities } from "@/modules/github/lib/capabilities";
import type { GHItem, ItemKind } from "@/modules/github/lib/items";
import { useRepoSlug } from "@/modules/github/lib/useRepoSlug";
import { openSettingsWindow } from "@/modules/settings/openSettingsWindow";
import { GithubIcon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useEffect, useState } from "react";
import { CommitBox } from "./CommitBox";
import { CreateItemView } from "./CreateItemView";
import { ItemDetailView } from "./ItemDetailView";
import { ItemListView } from "./ItemListView";

type Props = {
  repoRoot: string;
  onOpenDiff: (input: {
    path: string;
    repoRoot: string;
    mode: "+" | "-";
    originalPath?: string | null;
  }) => void;
};

type View =
  | { mode: "list" }
  | { mode: "detail"; kind: ItemKind; number: number }
  | { mode: "create"; kind: ItemKind };

/**
 * Workspace tab that turns a repo's GitHub presence into a full hub: browse and
 * filter pull requests and issues, open them inline (body, comments, actions),
 * create new ones, and commit local changes — all without leaving ALTAI.
 */
export function GitHubItemsPanel({ repoRoot, onOpenDiff }: Props) {
  const connection = useGitHubStore((s) => s.connection);
  const githubHydrated = useGitHubStore((s) => s.hydrated);
  const refreshGitHub = useGitHubStore((s) => s.refresh);
  const slugState = useRepoSlug(repoRoot);
  const [view, setView] = useState<View>({ mode: "list" });
  const [kind, setKind] = useState<ItemKind>("pulls");
  // Bumped after a mutation (close/merge/comment/create) to refresh the list.
  const [reloadKey, setReloadKey] = useState(0);
  const slug = slugState.status === "ready" ? slugState.slug : null;
  const capabilities = githubCapabilities({
    connected: !!connection,
    repoState: slugState.status,
  });
  const workspaceName =
    repoRoot.split(/[\\/]/).filter(Boolean).pop() ?? "Local workspace";

  useEffect(() => {
    if (!githubHydrated) void refreshGitHub();
  }, [githubHydrated, refreshGitHub]);

  if (capabilities.remoteItems && slug && view.mode === "detail") {
    return (
      <div className="flex h-full w-full flex-col">
        <ItemDetailView
          slug={slug}
          kind={view.kind}
          number={view.number}
          onBack={() => setView({ mode: "list" })}
          onMutated={() => setReloadKey((k) => k + 1)}
        />
      </div>
    );
  }

  if (capabilities.remoteMutations && slug && view.mode === "create") {
    return (
      <div className="flex h-full w-full flex-col">
        <CreateItemView
          slug={slug}
          kind={view.kind}
          onBack={() => setView({ mode: "list" })}
          onCreated={(item: GHItem) => {
            setReloadKey((k) => k + 1);
            setView({ mode: "detail", kind: view.kind, number: item.number });
          }}
        />
      </div>
    );
  }

  return (
    <div className="mx-auto flex h-full w-full max-w-2xl flex-col gap-3 px-4 py-3">
      {/* Repo header */}
      <div className="flex items-center gap-2">
        <HugeiconsIcon
          icon={GithubIcon}
          size={16}
          strokeWidth={1.75}
          className="shrink-0 text-muted-foreground"
        />
        <span className="min-w-0 flex-1 truncate text-[13px] font-semibold text-foreground">
          {slug ? `${slug.owner}/${slug.repo}` : workspaceName}
        </span>
        <span className="rounded-full bg-foreground/[0.06] px-2 py-0.5 text-[9.5px] font-medium uppercase tracking-wide text-muted-foreground">
          Local Git always available
        </span>
      </div>

      <CommitBox repoRoot={repoRoot} onOpenDiff={onOpenDiff} />

      {!githubHydrated ? (
        <RemoteNotice
          title="Checking GitHub connection"
          description="Local Git remains available while ALTAI checks the optional GitHub integration."
          action={<Spinner className="size-4" />}
        />
      ) : !connection ? (
        <RemoteNotice
          title="Connect GitHub to load remote items"
          description="Local changes and commits remain available above. Connect only when you want to browse or modify issues and pull requests."
          action={
            <Button
              size="sm"
              className="h-8 gap-1.5 text-[11.5px]"
              onClick={() => openSettingsWindow("github")}
            >
              <HugeiconsIcon icon={GithubIcon} size={13} strokeWidth={1.8} />
              Connect GitHub
            </Button>
          }
        />
      ) : slugState.status === "loading" ? (
        <RemoteNotice
          title="Resolving GitHub repository"
          description="Local Git remains available while ALTAI checks the origin remote."
          action={<Spinner className="size-4" />}
        />
      ) : !slug ? (
        <RemoteNotice
          title="No GitHub origin found"
          description="Local changes and commits remain available. Add a GitHub origin to browse issues and pull requests."
        />
      ) : (
        <ItemListView
          slug={slug}
          kind={kind}
          onKindChange={setKind}
          onOpenItem={(k, number) =>
            setView({ mode: "detail", kind: k, number })
          }
          onCreate={(k) => setView({ mode: "create", kind: k })}
          reloadKey={reloadKey}
        />
      )}
    </div>
  );
}

function RemoteNotice({
  title,
  description,
  action,
}: {
  title: string;
  description: string;
  action?: React.ReactNode;
}) {
  return (
    <div className="flex min-h-32 items-center gap-3 rounded-xl border border-border/60 bg-card/30 px-4 py-4">
      <span className="flex size-9 shrink-0 items-center justify-center rounded-xl bg-foreground/[0.04] text-muted-foreground">
        <HugeiconsIcon icon={GithubIcon} size={18} strokeWidth={1.7} />
      </span>
      <div className="min-w-0 flex-1">
        <p className="text-[12.5px] font-medium text-foreground">{title}</p>
        <p className="mt-0.5 max-w-xl text-[11px] leading-relaxed text-muted-foreground">
          {description}
        </p>
      </div>
      {action ? <div className="shrink-0">{action}</div> : null}
    </div>
  );
}
