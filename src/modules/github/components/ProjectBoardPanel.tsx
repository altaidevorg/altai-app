import { useGitHubStore } from "@/modules/github";
import { githubCapabilities } from "@/modules/github/lib/capabilities";
import {
  listRepoProjects,
  type ProjectSummary,
} from "@/modules/github/lib/projects";
import { useRepoSlug } from "@/modules/github/lib/useRepoSlug";
import { GithubIcon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useEffect, useState } from "react";
import { OverviewBoard } from "./OverviewBoard";
import { ProjectsV2Board } from "./ProjectsV2Board";

type Props = {
  repoRoot: string;
};

const OVERVIEW = "overview";

function isScopeError(message: string): boolean {
  const m = message.toLowerCase();
  return (
    m.includes("required scope") ||
    m.includes("requires the project") ||
    (m.includes("scope") &&
      (m.includes("project") || m.includes("read:project")))
  );
}

/**
 * Local-first project-management tab. Overview is always available for todos
 * and agent runs. Authentication only unlocks remote issues, pull requests,
 * and linked GitHub Projects.
 */
export function ProjectBoardPanel({ repoRoot }: Props) {
  const connection = useGitHubStore((s) => s.connection);
  const githubHydrated = useGitHubStore((s) => s.hydrated);
  const refreshGitHub = useGitHubStore((s) => s.refresh);
  const slugState = useRepoSlug(repoRoot);
  const [projects, setProjects] = useState<ProjectSummary[]>([]);
  const [projectsNote, setProjectsNote] = useState<string | null>(null);
  const [mode, setMode] = useState<string>(OVERVIEW);

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

  // Fetch linked Projects v2 for the mode selector. Non-blocking: a scope error
  // or no projects must NOT stop the Overview board from rendering.
  useEffect(() => {
    if (!slug || !capabilities.linkedProjects) {
      setProjects([]);
      setProjectsNote(null);
      setMode(OVERVIEW);
      return;
    }
    let alive = true;
    setProjectsNote(null);
    listRepoProjects(slug)
      .then((list) => {
        if (alive) setProjects(list);
      })
      .catch((e: unknown) => {
        if (!alive) return;
        const msg = e instanceof Error ? e.message : String(e);
        setProjectsNote(
          isScopeError(msg)
            ? "Reconnect with the project scope to use GitHub Projects boards."
            : msg,
        );
      });
    return () => {
      alive = false;
    };
  }, [slug, capabilities.linkedProjects]);

  return (
    <div className="flex h-full w-full flex-col">
      {/* Header: repo + mode selector */}
      <div className="flex shrink-0 items-center gap-2 border-b border-border/50 px-4 py-2.5">
        <HugeiconsIcon
          icon={GithubIcon}
          size={15}
          strokeWidth={1.75}
          className="shrink-0 text-muted-foreground"
        />
        <span className="shrink-0 truncate text-[12.5px] font-medium text-foreground">
          {slug ? `${slug.owner}/${slug.repo}` : workspaceName}
        </span>
        <span className="text-muted-foreground/40">/</span>
        <select
          value={mode}
          onChange={(e) => setMode(e.target.value)}
          aria-label="Board view"
          className="h-7 max-w-[16rem] rounded-lg border border-border/60 bg-background/60 px-2 text-[12px] font-medium text-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
        >
          <option value={OVERVIEW}>Overview</option>
          {projects.length > 0 ? (
            <optgroup label="GitHub Projects">
              {projects.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.title}
                  {p.closed ? " (closed)" : ""}
                </option>
              ))}
            </optgroup>
          ) : null}
        </select>
        {mode === OVERVIEW && projectsNote ? (
          <span className="truncate text-[10.5px] text-muted-foreground/55">
            {projectsNote}
          </span>
        ) : null}
      </div>

      <div className="min-h-0 flex-1">
        {mode === OVERVIEW || !slug || !capabilities.linkedProjects ? (
          <OverviewBoard
            repoRoot={repoRoot}
            slug={slug}
            githubConnected={!!connection}
            githubConnectionReady={githubHydrated}
            repoState={slugState.status}
          />
        ) : (
          <ProjectsV2Board key={mode} projectId={mode} slug={slug} />
        )}
      </div>
    </div>
  );
}
