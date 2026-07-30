import {
  ArrowRight01Icon,
  Cancel01Icon,
  Clock01Icon,
  FolderOpenIcon,
  GitForkIcon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { homeDir } from "@tauri-apps/api/path";
import { useEffect, useRef, useState } from "react";
import { Spinner } from "@/components/ui/spinner";
import { cn } from "@/lib/utils";
import { useAppMenuCommands } from "@/modules/app-menu/useAppMenuCommands";
import { folderName, prettyDir, useWorkspaceFolderStore } from "./folder";

/**
 * Agent-first project picker. A project supplies the durable scope for chats,
 * runs, permissions, worktrees, and optional Studio tools.
 */
export function WorkspaceWelcome() {
  const recents = useWorkspaceFolderStore((s) => s.recents);
  const pickFolder = useWorkspaceFolderStore((s) => s.pickFolder);
  const openRecent = useWorkspaceFolderStore((s) => s.openRecent);
  const removeRecent = useWorkspaceFolderStore((s) => s.removeRecent);
  const cloneRepo = useWorkspaceFolderStore((s) => s.cloneRepo);
  const [picking, setPicking] = useState(false);
  const [openingRecent, setOpeningRecent] = useState<string | null>(null);
  const [home, setHome] = useState<string | null>(null);

  // Clone-repo inline form state.
  const [cloneOpen, setCloneOpen] = useState(false);
  const [cloneUrl, setCloneUrl] = useState("");
  const [cloning, setCloning] = useState(false);
  const [cloneError, setCloneError] = useState<string | null>(null);

  useAppMenuCommands((command) => {
    if (command.id === "file.openFolder") {
      void pickFolder();
    } else if (command.id === "file.openRecent" && command.path) {
      void openRecent(command.path);
    }
  });

  // Move SR focus into the title on mount so screen-reader users land in
  // context instead of at the top of an unlabelled document.
  const titleRef = useRef<HTMLHeadingElement | null>(null);
  useEffect(() => {
    titleRef.current?.focus();
  }, []);

  useEffect(() => {
    let alive = true;
    homeDir()
      .then((h) => {
        if (alive) setHome(h.replace(/[/\\]+$/, ""));
      })
      .catch(() => {});
    return () => {
      alive = false;
    };
  }, []);

  const onOpen = async () => {
    setPicking(true);
    try {
      await pickFolder();
    } finally {
      setPicking(false);
    }
  };

  const onClone = async () => {
    if (cloning) return;
    setCloneError(null);
    setCloning(true);
    try {
      // On success the workspace folder is set → this screen unmounts. A null
      // result means the user cancelled the destination dialog: stay put.
      await cloneRepo(cloneUrl);
    } catch (e) {
      setCloneError(e instanceof Error ? e.message : String(e));
    } finally {
      setCloning(false);
    }
  };

  return (
    <main
      aria-labelledby="workspace-welcome-title"
      className="relative flex h-screen w-screen items-center justify-center overflow-hidden bg-background px-6 text-foreground"
    >
      <div className="pointer-events-none absolute inset-x-[18%] top-[-12rem] h-72 rounded-full bg-primary/[0.055] blur-3xl" />
      <div className="relative flex w-full max-w-md flex-col items-center rounded-2xl border border-border-subtle bg-card/65 px-7 py-8 shadow-xl backdrop-blur">
        {/* Brand */}
        <img
          src="/logo.png"
          alt="ALTAI"
          draggable={false}
          className="size-14 rounded-2xl"
        />
        <div className="mt-4 text-[10px] font-semibold uppercase tracking-[0.16em] text-muted-foreground">
          Agent workspace
        </div>
        <h1
          id="workspace-welcome-title"
          ref={titleRef}
          tabIndex={-1}
          className="mt-1.5 text-xl font-semibold tracking-tight outline-none"
        >
          Start with a project
        </h1>
        <p className="mt-1 max-w-sm text-center text-[12.5px] leading-relaxed text-muted-foreground">
          Chats, agent runs, permissions, and worktrees stay organized around
          the project you choose.
        </p>

        {/* Primary actions */}
        <div className="mt-8 flex w-full flex-col gap-1">
          <ActionRow
            icon={FolderOpenIcon}
            title="Open local project"
            hint="Choose a folder and start in Agent Workspace"
            onClick={() => void onOpen()}
            disabled={picking}
            loading={picking}
          />
          <ActionRow
            icon={GitForkIcon}
            title="Clone repository"
            hint="Bring a Git project into a new workspace"
            onClick={() => setCloneOpen((v) => !v)}
            active={cloneOpen}
            disabled={cloning}
          />
          {cloneOpen ? (
            <div className="mb-1 ml-12 mr-1 flex flex-col gap-2 rounded-lg border border-border/60 bg-card/40 p-2.5">
              <input
                type="text"
                value={cloneUrl}
                onChange={(e) => setCloneUrl(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") void onClone();
                }}
                placeholder="https://github.com/user/repo.git"
                aria-label="Repository URL"
                autoFocus
                spellCheck={false}
                disabled={cloning}
                className={cn(
                  "w-full rounded-md border border-border/60 bg-background px-2.5 py-1.5",
                  "font-mono text-[12px] text-foreground placeholder:text-muted-foreground/50",
                  "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
                  "disabled:opacity-60",
                )}
              />
              {cloneError ? (
                <div
                  role="alert"
                  className="rounded bg-destructive/10 px-2 py-1.5 text-[11px] leading-relaxed text-destructive"
                >
                  {cloneError}
                </div>
              ) : null}
              <span role="status" className="sr-only">
                {cloning ? "Cloning repository…" : ""}
              </span>
              <button
                type="button"
                onClick={() => void onClone()}
                disabled={cloning || cloneUrl.trim().length === 0}
                className={cn(
                  "flex items-center justify-center gap-1.5 rounded-md bg-primary px-3 py-1.5",
                  "text-[12px] font-medium text-primary-foreground transition-colors",
                  "hover:bg-primary/90 disabled:opacity-50",
                )}
              >
                {cloning ? (
                  <>
                    <Spinner className="size-3.5" />
                    Cloning…
                  </>
                ) : (
                  "Choose location & clone"
                )}
              </button>
            </div>
          ) : null}
        </div>

        {/* Recent projects */}
        <div className="mt-8 flex w-full flex-col">
          <h2
            id="workspace-recents-title"
            className="mb-2 flex items-center justify-center gap-1.5 text-[11px] font-medium uppercase tracking-wide text-muted-foreground/70"
          >
            <HugeiconsIcon icon={Clock01Icon} size={12} strokeWidth={1.75} />
            Recent projects
          </h2>
          {recents.length === 0 ? (
            <div className="rounded-lg border border-dashed border-border/60 px-3 py-5 text-center text-[12px] text-muted-foreground/70">
              No recent projects yet.
            </div>
          ) : (
            <ul
              aria-labelledby="workspace-recents-title"
              className="flex min-w-0 flex-col"
            >
              {recents.map((path) => (
                <li key={path} className="group/recent min-w-0">
                  <div className="flex min-w-0 items-center rounded-md transition-colors hover:bg-muted/50">
                    <button
                      type="button"
                      disabled={openingRecent !== null || picking || cloning}
                      onClick={async () => {
                        if (openingRecent || picking || cloning) return;
                        setOpeningRecent(path);
                        try {
                          await openRecent(path);
                        } finally {
                          setOpeningRecent(null);
                        }
                      }}
                      className="flex min-w-0 flex-1 items-center gap-1.5 px-2 py-1.5 text-left focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:opacity-50"
                    >
                      <span className="min-w-0 shrink-0 truncate text-[12.5px] font-medium text-foreground">
                        {folderName(path)}
                      </span>
                      <span className="min-w-0 flex-1 truncate font-mono text-[11px] text-muted-foreground/70">
                        {prettyDir(path, home)}
                      </span>
                    </button>
                    <button
                      type="button"
                      onClick={() => removeRecent(path)}
                      aria-label={`Remove ${folderName(path)} from recents`}
                      className="mr-1 flex size-5 shrink-0 items-center justify-center rounded text-muted-foreground/50 opacity-0 transition-opacity hover:bg-muted hover:text-foreground focus-visible:opacity-100 group-hover/recent:opacity-100"
                    >
                      <HugeiconsIcon
                        icon={Cancel01Icon}
                        size={12}
                        strokeWidth={2}
                      />
                    </button>
                  </div>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>
    </main>
  );
}

function ActionRow({
  icon,
  title,
  hint,
  onClick,
  disabled = false,
  loading = false,
  soon = false,
  active = false,
}: {
  icon: typeof FolderOpenIcon;
  title: string;
  hint: string;
  onClick?: () => void;
  disabled?: boolean;
  loading?: boolean;
  soon?: boolean;
  active?: boolean;
}) {
  const inert = soon || disabled;
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={inert}
      title={soon ? "Coming soon" : undefined}
      className={cn(
        "group/action flex items-center gap-3 rounded-lg px-2.5 py-2.5 text-left transition-colors",
        "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
        soon
          ? "cursor-default opacity-55"
          : "hover:bg-muted/60 disabled:opacity-60",
        active && "bg-muted/60",
      )}
    >
      <span className="flex size-9 shrink-0 items-center justify-center rounded-lg bg-foreground/[0.05] text-muted-foreground transition-colors group-hover/action:text-foreground">
        <HugeiconsIcon icon={icon} size={17} strokeWidth={1.75} />
      </span>
      <span className="flex min-w-0 flex-1 flex-col">
        <span className="flex items-center gap-2 text-[13px] font-medium text-foreground">
          {loading ? "Opening…" : title}
          {soon ? (
            <span className="rounded-full border border-border/60 px-1.5 py-px text-[9.5px] font-medium uppercase tracking-wide text-muted-foreground/70">
              Soon
            </span>
          ) : null}
        </span>
        <span className="truncate text-[11.5px] text-muted-foreground/80">
          {hint}
        </span>
      </span>
      {!soon ? (
        <HugeiconsIcon
          icon={ArrowRight01Icon}
          size={14}
          strokeWidth={2}
          className="shrink-0 text-muted-foreground/40 transition-transform group-hover/action:translate-x-0.5 group-hover/action:text-muted-foreground"
        />
      ) : null}
    </button>
  );
}
