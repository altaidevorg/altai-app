import { FolderOpenIcon, GithubIcon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";

export type WorkspaceTargetBusy = "local" | "github" | null;

export type WorkspaceTargetFormProps = {
  busy?: WorkspaceTargetBusy;
  error?: string | null;
  repoUrl: string;
  onRepoUrlChange: (value: string) => void;
  canChooseLocal?: boolean;
  canCloneGithub?: boolean;
  showClearProject?: boolean;
  onChooseLocal: () => void;
  onCloneGithub: () => void;
  onClearProject?: () => void;
};

/**
 * Presentational body for choosing a chat project target (local folder, GitHub
 * clone, or no project). Dialog/modal chrome stays on the host.
 */
export function WorkspaceTargetForm({
  busy = null,
  error = null,
  repoUrl,
  onRepoUrlChange,
  canChooseLocal = true,
  canCloneGithub = true,
  showClearProject = false,
  onChooseLocal,
  onCloneGithub,
  onClearProject,
}: WorkspaceTargetFormProps) {
  return (
    <div className="altai-workspace-target-form space-y-4">
      <div className="space-y-1.5">
        <h2 className="text-base font-semibold leading-none tracking-tight">
          Choose a project
        </h2>
        <p className="text-sm text-muted-foreground">
          Keep the conversation project-free, attach a local folder, or clone a
          GitHub repository. ALTAI only receives file context after you choose a
          project target.
        </p>
      </div>

      <div className="space-y-2">
        <button
          type="button"
          onClick={onChooseLocal}
          disabled={!canChooseLocal || busy !== null}
          className="flex w-full items-center gap-3 rounded-xl border border-border bg-card px-3.5 py-3 text-left transition-colors hover:bg-accent disabled:opacity-50"
        >
          <span className="inline-flex size-9 shrink-0 items-center justify-center rounded-lg bg-muted text-muted-foreground">
            <HugeiconsIcon icon={FolderOpenIcon} size={17} strokeWidth={1.75} />
          </span>
          <span className="min-w-0 flex-1">
            <span className="block text-[12.5px] font-medium text-foreground">
              {busy === "local" ? "Opening…" : "Local workspace"}
            </span>
            <span className="mt-0.5 block text-[10.5px] text-muted-foreground">
              Choose a folder only for chats that need local files and tools.
            </span>
          </span>
        </button>

        <div className="rounded-xl border border-border bg-card p-3.5">
          <div className="flex items-center gap-3">
            <span className="inline-flex size-9 shrink-0 items-center justify-center rounded-lg bg-muted text-muted-foreground">
              <HugeiconsIcon icon={GithubIcon} size={17} strokeWidth={1.75} />
            </span>
            <span className="min-w-0 flex-1">
              <span className="block text-[12.5px] font-medium text-foreground">
                GitHub repository
              </span>
              <span className="mt-0.5 block text-[10.5px] text-muted-foreground">
                Clone a repository and attach the resulting isolated workspace.
              </span>
            </span>
          </div>
          <div className="mt-3 flex min-w-0 gap-2">
            <input
              value={repoUrl}
              onChange={(event) => onRepoUrlChange(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter") onCloneGithub();
              }}
              placeholder="https://github.com/org/repository.git"
              aria-label="GitHub repository URL"
              className="h-8 min-w-0 flex-1 rounded-lg border border-border bg-background px-2.5 font-mono text-[10.5px] text-foreground outline-none placeholder:text-muted-foreground/60 focus:border-ring"
            />
            <button
              type="button"
              onClick={onCloneGithub}
              disabled={
                !canCloneGithub || busy !== null || !repoUrl.trim()
              }
              className="h-8 shrink-0 rounded-lg bg-primary px-3 text-[10.5px] font-medium text-primary-foreground transition-colors hover:bg-primary/90 disabled:opacity-50"
            >
              {busy === "github" ? "Cloning…" : "Clone"}
            </button>
          </div>
        </div>

        {showClearProject && onClearProject ? (
          <button
            type="button"
            onClick={onClearProject}
            disabled={busy !== null}
            className="w-full rounded-xl border border-border px-3.5 py-2.5 text-left text-[11px] text-muted-foreground transition-colors hover:bg-accent hover:text-foreground disabled:opacity-50"
          >
            Continue without a project
          </button>
        ) : null}
      </div>

      {error ? (
        <div
          role="alert"
          className="rounded-lg border border-destructive/30 bg-destructive/[0.06] px-3 py-2 text-[10.5px] text-destructive"
        >
          {error}
        </div>
      ) : null}
    </div>
  );
}
