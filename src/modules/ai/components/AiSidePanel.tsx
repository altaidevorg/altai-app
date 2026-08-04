import { hasTauriWindowMetadata } from "@/lib/tauriWindow";
import { cn } from "@/lib/utils";
import { IS_MAC, USE_CUSTOM_WINDOW_CONTROLS } from "@/lib/platform";
import { WindowControls } from "@/components/WindowControls";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import {
  Add01Icon,
  ArrowDown01Icon,
  Cancel01Icon,
  Clock01Icon,
  CodeIcon,
  FileEditIcon,
  FolderOpenIcon,
  Folder01Icon,
  GithubIcon,
  Notebook01Icon,
  Notification01Icon,
  Settings01Icon,
  SparklesIcon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { type ReactElement, useEffect, useRef, useState } from "react";
import {
  AgentsInspector,
  EditApprovalCard,
  InspectorEmpty,
  InspectorMetric,
  RunStateMetric,
  SurfaceHeader,
  SurfaceSearch,
  TodosInspector,
} from "@altai/agent-ui";
import {
  retryFailedRun,
  sendMessage,
  stop as stopAgent,
  useChatStore,
} from "../store/chatStore";
import { useAgentRunsStore } from "../store/agentRunsStore";
import {
  continueBudgetSegmentPrompt,
  continueStuckPrompt,
  describeRunWarning,
  dismissRunAttention,
  isRetryableRunOutcome,
} from "../lib/agentEventBridge";
import { useAgentsStore } from "../store/agentsStore";
import { usePlanStore, type AppliedPlanEdit } from "../store/planStore";
import { useTodosStore } from "../store/todoStore";
import { native, type CheckpointInfo } from "../lib/native";
import { openSettingsWindow } from "@/modules/settings/openSettingsWindow";
import { AiChatView } from "./AiChat";
import { AiInputBar, AiInputBarConnect } from "./AiInputBar";
import { AgentStatusPill } from "./AgentStatusPill";
import { ChatHistoryPanel } from "./ChatHistoryPanel";
import { PlanDiffReview } from "./PlanDiffReview";
import { NotificationInboxPanel } from "./NotificationInboxPanel";
import { TodoSummaryChip } from "./TodoStrip";
import { WorkHubPanel, type WorkHubView } from "./WorkHubPanel";
import {
  selectNotificationAttentionCount,
  useNotificationStore,
} from "../store/notificationStore";

// Zustand selectors must return a stable reference when a session has no
// todos yet; allocating `[]` inside the selector triggers React's external
// store loop detector and can blank the whole renderer.
const EMPTY_TODOS: Array<{ id: string; title: string; status: string }> = [];
type PanelSurface = "history" | "inspector" | "work" | "inbox" | null;
const HISTORY_PANEL_WIDTH_KEY = "altai.ai.historyPanel.width";
const INSPECTOR_PANEL_WIDTH_KEY = "altai.ai.inspectorPanel.width";
const HISTORY_PANEL_MIN_WIDTH = 176;
const HISTORY_PANEL_MAX_WIDTH = 360;
const INSPECTOR_PANEL_MIN_WIDTH = 240;
const INSPECTOR_PANEL_MAX_WIDTH = 480;

function readPanelWidth(
  key: string,
  fallback: number,
  min: number,
  max: number,
): number {
  try {
    const parsed = Number.parseInt(window.localStorage.getItem(key) ?? "", 10);
    return Number.isFinite(parsed)
      ? Math.min(max, Math.max(min, parsed))
      : fallback;
  } catch {
    return fallback;
  }
}

function persistPanelWidth(key: string, width: number, min: number, max: number) {
  if (width <= 0) return;
  try {
    window.localStorage.setItem(
      key,
      String(Math.round(Math.min(max, Math.max(min, width)))),
    );
  } catch {
    // Storage can be unavailable in restricted webviews; resizing still works.
  }
}

export type AiSidePanelProps = {
  onClose?: () => void;
  hasComposer?: boolean;
  variant?: "workspace" | "sidebar";
  workspaceName?: string;
  workspacePath?: string | null;
  workspaceKind?: "local" | "github" | null;
  onOpenStudio?: () => void;
  onOpenSettings?: () => void;
  onChooseLocalWorkspace?: () => Promise<string | null>;
  onCloneGithubRepository?: (url: string) => Promise<string | null>;
  onClearWorkspace?: () => void;
};

export function AiSidePanel({
  onClose,
  hasComposer = true,
  variant = "sidebar",
  workspaceName = "Local workspace",
  workspacePath = null,
  workspaceKind = null,
  onOpenStudio,
  onOpenSettings,
  onChooseLocalWorkspace,
  onCloneGithubRepository,
  onClearWorkspace,
}: AiSidePanelProps) {
  const sessionId = useChatStore((s) => s.activeSessionId);
  const chatSessions = useChatStore((s) => s.sessions);
  const switchSession = useChatStore((s) => s.switchSession);
  const newSession = useChatStore((s) => s.newSession);

  useEffect(() => {
    if (!onClose) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      const target = e.target as HTMLElement | null;
      const tag = target?.tagName;
      if (tag === "INPUT" || tag === "TEXTAREA") return;
      // Don't compete with Radix popovers/menus/dialogs — their own
      // dismiss handlers should run first. Radix sets data-state="open"
      // on triggers and renders portaled overlays with role="menu" /
      // role="listbox" / role="dialog".
      if (target?.closest('[data-state="open"]')) return;
      if (
        document.querySelector(
          '[role="menu"][data-state="open"], [role="listbox"][data-state="open"], [role="dialog"][data-state="open"]',
        )
      ) {
        return;
      }
      onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  const [activeSurface, setActiveSurface] = useState<PanelSurface>(null);
  const [workView, setWorkView] = useState<WorkHubView>("runs");
  const [openChatIds, setOpenChatIds] = useState<string[]>([]);
  const [reviewOpen, setReviewOpen] = useState(false);
  const [targetDialogOpen, setTargetDialogOpen] = useState(false);
  const panelRootRef = useRef<HTMLElement | null>(null);
  const [panelWidth, setPanelWidth] = useState(0);
  const historyOpen = activeSurface === "history";
  const inspectorOpen = activeSurface === "inspector";
  const workOpen = activeSurface === "work";
  const inboxOpen = activeSurface === "inbox";
  // Run details is a stable destination for the active chat. Keep its control
  // visible while History, Work, Inbox, or Review is open so switching surfaces
  // never causes the toolbar geometry to jump.
  const inspectorAvailable = Boolean(sessionId);
  // A persistent history rail belongs only to the standalone Agent Workspace.
  // Inside the IDE, widening the chat must never introduce a second left
  // sidebar; history remains an explicit, single-surface destination.
  const showHistorySidebar = variant === "workspace" && panelWidth >= 768;
  const showInspectorSidebar =
    panelWidth >= 1216 && inspectorOpen && inspectorAvailable;
  const toggleSurface = (surface: Exclude<PanelSurface, null>) => {
    setReviewOpen(false);
    setActiveSurface((current) => (current === surface ? null : surface));
  };

  useEffect(() => {
    const root = panelRootRef.current;
    if (!root || typeof ResizeObserver === "undefined") return;
    const updateWidth = () => setPanelWidth(Math.round(root.getBoundingClientRect().width));
    updateWidth();
    const observer = new ResizeObserver((entries) => {
      const width = entries[0]?.contentRect.width;
      if (typeof width === "number") setPanelWidth(Math.round(width));
    });
    observer.observe(root);
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    const openSurface = (event: Event) => {
      const detail = (event as CustomEvent<{
        surface?: string;
        view?: WorkHubView;
      }>).detail;
      const surface = detail?.surface;
      if (surface === "review") {
        setActiveSurface(null);
        setReviewOpen(true);
        return;
      }
      if (
        surface === "history" ||
        surface === "inspector" ||
        surface === "inbox" ||
        surface === "work"
      ) {
        setReviewOpen(false);
        setActiveSurface(surface);
        if (surface === "work") setWorkView(detail?.view ?? "runs");
        return;
      }
      // Keep deep links from older slash-command and extension surfaces
      // working while the public destination is consolidated under Work.
      if (surface === "tasks" || surface === "automations") {
        setReviewOpen(false);
        setWorkView(surface === "automations" ? "scheduled" : "runs");
        setActiveSurface("work");
      }
    };
    window.addEventListener("altai:open-ai-surface", openSurface);
    return () => window.removeEventListener("altai:open-ai-surface", openSurface);
  }, []);

  // Session history and open chat tabs are deliberately separate. Selecting a
  // conversation from history opens it in a tab; closing that tab keeps the
  // local conversation available in history instead of deleting it.
  useEffect(() => {
    setOpenChatIds((current) => {
      const valid = current.filter((id) => chatSessions.some((session) => session.id === id));
      if (sessionId && !valid.includes(sessionId)) valid.push(sessionId);
      return valid;
    });
  }, [chatSessions, sessionId]);

  const createChatTab = () => {
    const id = newSession();
    setOpenChatIds((current) => (current.includes(id) ? current : [...current, id]));
    setActiveSurface(null);
  };

  const closeChatTab = (chatId: string) => {
    const index = openChatIds.indexOf(chatId);
    if (index < 0) return;
    const remaining = openChatIds.filter((id) => id !== chatId);
    if (remaining.length === 0) {
      const id = newSession();
      setOpenChatIds([id]);
      setActiveSurface(null);
      return;
    }
    if (sessionId === chatId) {
      switchSession(remaining[Math.min(index, remaining.length - 1)]);
    }
    setOpenChatIds(remaining);
    setActiveSurface(null);
  };

  useEffect(() => {
    const openReview = () => setReviewOpen(true);
    window.addEventListener("altai:open-change-review", openReview);
    return () => window.removeEventListener("altai:open-change-review", openReview);
  }, []);

  return (
    <aside
      ref={panelRootRef}
      data-ai-side-panel
      data-ai-workspace={variant === "workspace" ? "true" : undefined}
      id="altai-ai-panel"
      aria-label={variant === "workspace" ? "ALTAI agent workspace" : "AI assistant"}
      className="altai-ai-panel @container relative flex h-full min-h-0 overflow-hidden bg-card text-[12px]"
    >
      <div className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
        <WorkspaceTopbar
          variant={variant}
          workspacePath={workspacePath}
          onOpenStudio={onOpenStudio}
          onClose={onClose}
          openChatIds={openChatIds}
          onSelectChat={() => setActiveSurface(null)}
          onCloseChat={closeChatTab}
          onNewChat={createChatTab}
          historyOpen={historyOpen}
          onToggleHistory={() => toggleSurface("history")}
          inspectorOpen={inspectorOpen}
          inspectorAvailable={inspectorAvailable}
          onToggleInspector={() => toggleSurface("inspector")}
          workOpen={workOpen}
          onToggleWork={() => toggleSurface("work")}
          inboxOpen={inboxOpen}
          onToggleInbox={() => toggleSurface("inbox")}
          onOpenSettings={onOpenSettings}
        />
        <ResizablePanelGroup
          orientation="horizontal"
          className="relative isolate min-h-0 flex-1 overflow-hidden"
        >
          {showHistorySidebar ? (
            <>
              <ResizablePanel
                id="ai-history-sidebar"
                defaultSize={`${readPanelWidth(
                  HISTORY_PANEL_WIDTH_KEY,
                  216,
                  HISTORY_PANEL_MIN_WIDTH,
                  HISTORY_PANEL_MAX_WIDTH,
                )}px`}
                minSize={`${HISTORY_PANEL_MIN_WIDTH}px`}
                maxSize={`${HISTORY_PANEL_MAX_WIDTH}px`}
                onResize={(size) =>
                  persistPanelWidth(
                    HISTORY_PANEL_WIDTH_KEY,
                    size.inPixels,
                    HISTORY_PANEL_MIN_WIDTH,
                    HISTORY_PANEL_MAX_WIDTH,
                  )
                }
              >
                <nav
                  aria-label="Chat sessions"
                  className="altai-ai-history-rail z-10 flex h-full min-h-0 min-w-0 flex-col overflow-hidden"
                >
                  <ChatHistoryPanel
                    onClose={() => setActiveSurface(null)}
                    autoFocusSearch={historyOpen}
                  />
                </nav>
              </ResizablePanel>
              <ResizableHandle
                withHandle
                aria-label="Resize chat history sidebar"
                title="Resize chat history sidebar"
              />
            </>
          ) : null}

          <ResizablePanel id="ai-chat-main" minSize="240px">
            <main className="altai-ai-main relative z-0 flex h-full min-h-0 min-w-0 flex-col overflow-hidden bg-card">
              {historyOpen && !showHistorySidebar ? (
                <div className="flex min-h-0 flex-1">
                  <ChatHistoryPanel
                    autoFocusSearch
                    onClose={() => setActiveSurface(null)}
                  />
                </div>
              ) : null}
              {sessionId ? (
                <div
                  className={cn(
                    "relative min-h-0 flex-1",
                    historyOpen && !showHistorySidebar ? "hidden" : "flex",
                  )}
                >
                  <Body
                    hasComposer={hasComposer}
                    onOpenReview={() => {
                      setActiveSurface(null);
                      setReviewOpen(true);
                    }}
                    projectTarget={
                      variant === "workspace"
                        ? {
                            name: workspaceName,
                            path: workspacePath,
                            kind: workspaceKind,
                            onChange: () => setTargetDialogOpen(true),
                          }
                        : undefined
                    }
                  />
                  {workOpen ? (
                    <WorkHubPanel
                      initialView={workView}
                      onClose={() => setActiveSurface(null)}
                    />
                  ) : null}
                  {inboxOpen ? (
                    <NotificationInboxPanel onClose={() => setActiveSurface(null)} />
                  ) : null}
                  {inspectorOpen &&
                  inspectorAvailable &&
                  !showInspectorSidebar ? (
                    <div className="absolute inset-0 z-20 flex bg-card">
                      <RunInspector
                        className="flex w-full"
                        onClose={() => setActiveSurface(null)}
                      />
                    </div>
                  ) : null}
                  {reviewOpen ? (
                    <PlanDiffReview
                      open
                      autoOpen={false}
                      onClose={() => setReviewOpen(false)}
                    />
                  ) : null}
                </div>
              ) : (
                <div
                  className={cn(
                    "flex flex-1 items-center justify-center text-[11px] text-muted-foreground",
                    historyOpen && !showHistorySidebar && "hidden",
                  )}
                >
                  Loading sessions…
                </div>
              )}
            </main>
          </ResizablePanel>

          {showInspectorSidebar ? (
            <>
              <ResizableHandle
                withHandle
                aria-label="Resize run inspector sidebar"
                title="Resize run inspector sidebar"
              />
              <ResizablePanel
                id="ai-run-inspector-sidebar"
                defaultSize={`${readPanelWidth(
                  INSPECTOR_PANEL_WIDTH_KEY,
                  288,
                  INSPECTOR_PANEL_MIN_WIDTH,
                  INSPECTOR_PANEL_MAX_WIDTH,
                )}px`}
                minSize={`${INSPECTOR_PANEL_MIN_WIDTH}px`}
                maxSize={`${INSPECTOR_PANEL_MAX_WIDTH}px`}
                onResize={(size) =>
                  persistPanelWidth(
                    INSPECTOR_PANEL_WIDTH_KEY,
                    size.inPixels,
                    INSPECTOR_PANEL_MIN_WIDTH,
                    INSPECTOR_PANEL_MAX_WIDTH,
                  )
                }
              >
                <RunInspector
                  className="flex h-full w-full min-w-0 overflow-hidden"
                  onClose={() => setActiveSurface(null)}
                />
              </ResizablePanel>
            </>
          ) : null}
        </ResizablePanelGroup>
      </div>
      {variant === "workspace" ? (
        <WorkspaceTargetDialog
          open={targetDialogOpen}
          onOpenChange={setTargetDialogOpen}
          workspacePath={workspacePath}
          onChooseLocalWorkspace={onChooseLocalWorkspace}
          onCloneGithubRepository={onCloneGithubRepository}
          onClearWorkspace={onClearWorkspace}
        />
      ) : null}
    </aside>
  );
}

function WorkspaceTargetDialog({
  open,
  onOpenChange,
  workspacePath,
  onChooseLocalWorkspace,
  onCloneGithubRepository,
  onClearWorkspace,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  workspacePath: string | null;
  onChooseLocalWorkspace?: () => Promise<string | null>;
  onCloneGithubRepository?: (url: string) => Promise<string | null>;
  onClearWorkspace?: () => void;
}) {
  const [repoUrl, setRepoUrl] = useState("");
  const [busy, setBusy] = useState<"local" | "github" | null>(null);
  const [error, setError] = useState<string | null>(null);

  const chooseLocal = async () => {
    if (!onChooseLocalWorkspace || busy) return;
    setError(null);
    setBusy("local");
    try {
      const path = await onChooseLocalWorkspace();
      if (path) onOpenChange(false);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(null);
    }
  };

  const cloneGithub = async () => {
    if (!onCloneGithubRepository || busy || !repoUrl.trim()) return;
    setError(null);
    setBusy("github");
    try {
      const path = await onCloneGithubRepository(repoUrl.trim());
      if (path) {
        setRepoUrl("");
        onOpenChange(false);
      }
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(null);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="gap-4 rounded-2xl p-5 sm:max-w-[460px]">
        <DialogHeader>
          <DialogTitle>Choose a project</DialogTitle>
          <DialogDescription>
            Keep the conversation project-free, attach a local folder, or clone
            a GitHub repository. ALTAI only receives file context after you
            choose a project target.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-2">
          <button
            type="button"
            onClick={() => void chooseLocal()}
            disabled={!onChooseLocalWorkspace || busy !== null}
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
                onChange={(event) => setRepoUrl(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === "Enter") void cloneGithub();
                }}
                placeholder="https://github.com/org/repository.git"
                aria-label="GitHub repository URL"
                className="h-8 min-w-0 flex-1 rounded-lg border border-border bg-background px-2.5 font-mono text-[10.5px] text-foreground outline-none placeholder:text-muted-foreground/60 focus:border-ring"
              />
              <button
                type="button"
                onClick={() => void cloneGithub()}
                disabled={
                  !onCloneGithubRepository || busy !== null || !repoUrl.trim()
                }
                className="h-8 shrink-0 rounded-lg bg-primary px-3 text-[10.5px] font-medium text-primary-foreground transition-colors hover:bg-primary/90 disabled:opacity-50"
              >
                {busy === "github" ? "Cloning…" : "Clone"}
              </button>
            </div>
          </div>

          {workspacePath && onClearWorkspace ? (
            <button
              type="button"
              onClick={() => {
                onClearWorkspace();
                onOpenChange(false);
              }}
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
      </DialogContent>
    </Dialog>
  );
}

/**
 * Native `title` popovers are inconsistent in the desktop webview. Keep the
 * compact icon controls discoverable with the same Radix tooltip used by the
 * rest of the app instead.
 */
function IconTooltip({ label, children }: { label: string; children: ReactElement }) {
  return (
    <Tooltip delayDuration={350} disableHoverableContent>
      <TooltipTrigger asChild>{children}</TooltipTrigger>
      <TooltipContent side="bottom" sideOffset={6} className="text-[10.5px]">
        {label}
      </TooltipContent>
    </Tooltip>
  );
}

/**
 * Conversation tabs are a stable navigation layer, separate from workspace
 * actions and session history. Keeping the trailing new-chat action here
 * matches the mental model used by coding-agent chat editors: tabs switch
 * context; the header opens auxiliary surfaces.
 */
function ChatTabStrip({
  openChatIds,
  onSelect,
  onCloseChat,
  onNewChat,
  embedded = false,
}: {
  openChatIds: string[];
  onSelect: () => void;
  onCloseChat: (id: string) => void;
  onNewChat: () => void;
  embedded?: boolean;
}) {
  const activeId = useChatStore((s) => s.activeSessionId);
  const sessions = useChatStore((s) => s.sessions);
  const switchSession = useChatStore((s) => s.switchSession);
  const openSessions = openChatIds
    .map((id) => sessions.find((session) => session.id === id))
    .filter((session): session is NonNullable<typeof session> => Boolean(session));

  const select = (id: string) => {
    switchSession(id);
    onSelect();
  };

  return (
    <div
      className={cn(
        "altai-ai-chat-tabs flex h-10 min-w-0 items-center gap-1.5",
        embedded
          ? "flex-1 bg-transparent"
          : "shrink-0 border-b border-border-subtle bg-card px-2.5",
      )}
    >
      <div
        role="tablist"
        aria-label="Open chats"
        className="flex min-w-0 shrink items-center gap-1 overflow-x-auto"
      >
        {openSessions.map((session) => (
          <div
            key={session.id}
            className={cn(
              "group flex h-7 max-w-44 shrink-0 items-center rounded-lg border text-[10.5px] transition-colors",
              session.id === activeId
                ? "border-border bg-muted/70 font-medium text-foreground"
                : "border-transparent text-muted-foreground hover:border-border/60 hover:bg-accent hover:text-foreground",
            )}
          >
            <button
              id={`altai-chat-tab-${session.id}`}
              type="button"
              role="tab"
              aria-controls="altai-active-chat"
              aria-selected={session.id === activeId}
              onClick={() => select(session.id)}
              title={session.title || "New chat"}
              className="h-full min-w-0 truncate px-2.5 text-left outline-none"
            >
              {session.title || "New chat"}
            </button>
            <IconTooltip label={`Close ${session.title || "new chat"}`}>
              <button
                type="button"
                onClick={() => onCloseChat(session.id)}
                aria-label={`Close ${session.title || "new chat"}`}
                className="mr-1 inline-flex size-4 shrink-0 items-center justify-center rounded-md text-muted-foreground/70 hover:bg-foreground/[0.1] hover:text-foreground"
              >
                <HugeiconsIcon icon={Cancel01Icon} size={10} strokeWidth={2} />
              </button>
            </IconTooltip>
          </div>
        ))}
      </div>
      <IconTooltip label="New chat">
        <button
          type="button"
          onClick={onNewChat}
          aria-label="New chat"
          className="inline-flex size-7 shrink-0 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-foreground/[0.06] hover:text-foreground"
        >
          <HugeiconsIcon icon={Add01Icon} size={14} strokeWidth={1.75} />
        </button>
      </IconTooltip>
    </div>
  );
}

/**
 * The workspace topbar keeps the task context visible instead of treating the
 * chat as an isolated message list. Work and Inbox are durable destinations;
 * Run details is contextual to the current run.
 */
function WorkspaceTopbar({
  variant,
  workspacePath,
  onOpenStudio,
  onClose,
  openChatIds,
  onSelectChat,
  onCloseChat,
  onNewChat,
  historyOpen,
  onToggleHistory,
  inspectorOpen,
  inspectorAvailable,
  onToggleInspector,
  workOpen,
  onToggleWork,
  inboxOpen,
  onToggleInbox,
  onOpenSettings,
}: {
  variant: "workspace" | "sidebar";
  workspacePath: string | null;
  onOpenStudio?: () => void;
  onClose?: () => void;
  openChatIds: string[];
  onSelectChat: () => void;
  onCloseChat: (id: string) => void;
  onNewChat: () => void;
  historyOpen: boolean;
  onToggleHistory: () => void;
  inspectorOpen: boolean;
  inspectorAvailable: boolean;
  onToggleInspector: () => void;
  workOpen: boolean;
  onToggleWork: () => void;
  inboxOpen: boolean;
  onToggleInbox: () => void;
  onOpenSettings?: () => void;
}) {
  const activeId = useChatStore((s) => s.activeSessionId);
  const inboxAttentionCount = useNotificationStore(selectNotificationAttentionCount);
  const refreshInbox = useNotificationStore((s) => s.refresh);

  useEffect(() => {
    void refreshInbox(workspacePath);
  }, [refreshInbox, workspacePath]);

  const historyControl = (
    <IconTooltip label={historyOpen ? "Back to task" : "Chat sessions"}>
      <button
        type="button"
        onClick={onToggleHistory}
        aria-label={historyOpen ? "Back to task" : "Chat sessions"}
        aria-pressed={historyOpen}
        className={cn(
          "inline-flex size-7 shrink-0 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-foreground/[0.06] hover:text-foreground",
          variant === "workspace" && "@[48rem]:hidden",
          historyOpen && "bg-foreground/[0.09] text-foreground",
        )}
      >
        <HugeiconsIcon icon={Clock01Icon} size={14} strokeWidth={1.75} />
      </button>
    </IconTooltip>
  );

  const workspaceActions = (
    <div className="altai-ai-topbar-actions flex shrink-0 items-center gap-0.5 rounded-lg border border-border/60 bg-muted/35 p-0.5">
        <IconTooltip label={workOpen ? "Close work" : "Open work"}>
          <button
            type="button"
            onClick={onToggleWork}
            aria-label={workOpen ? "Close work" : "Open work"}
            aria-pressed={workOpen}
            className={cn(
              "inline-flex h-7 shrink-0 items-center justify-center gap-1.5 rounded-md px-1.5 text-muted-foreground transition-colors hover:bg-foreground/[0.06] hover:text-foreground",
              workOpen && "bg-foreground/[0.09] text-foreground",
            )}
          >
            <HugeiconsIcon icon={Notebook01Icon} size={14} strokeWidth={1.75} />
            {variant === "workspace" ? (
              <span className="hidden pr-0.5 text-[10px] font-medium @[40rem]:inline">
                Work
              </span>
            ) : null}
          </button>
        </IconTooltip>
        <IconTooltip
          label={
            inboxOpen
              ? "Close inbox"
              : inboxAttentionCount
                ? `Open inbox, ${inboxAttentionCount} need attention`
                : "Open inbox"
          }
        >
          <button
            type="button"
            onClick={onToggleInbox}
            aria-label={
              inboxOpen
                ? "Close inbox"
                : inboxAttentionCount
                  ? `Open inbox, ${inboxAttentionCount} need attention`
                  : "Open inbox"
            }
            aria-pressed={inboxOpen}
            className={cn(
              "relative inline-flex h-7 shrink-0 items-center justify-center gap-1.5 rounded-md px-1.5 text-muted-foreground transition-colors hover:bg-foreground/[0.06] hover:text-foreground",
              inboxOpen && "bg-foreground/[0.09] text-foreground",
            )}
          >
            <HugeiconsIcon icon={Notification01Icon} size={14} strokeWidth={1.75} />
            {variant === "workspace" ? (
              <span className="hidden pr-0.5 text-[10px] font-medium @[40rem]:inline">
                Inbox
              </span>
            ) : null}
            {inboxAttentionCount ? (
              <span className="absolute -right-1 -top-1 flex min-w-3.5 items-center justify-center rounded-full bg-warning px-1 text-[8px] font-semibold leading-3 text-warning-foreground">
                {inboxAttentionCount > 99 ? "99+" : inboxAttentionCount}
              </span>
            ) : null}
          </button>
        </IconTooltip>
        {inspectorAvailable ? (
          <IconTooltip label={inspectorOpen ? "Close run details" : "Open run details"}>
            <button
              type="button"
              onClick={onToggleInspector}
              aria-label={inspectorOpen ? "Close run details" : "Open run details"}
              aria-pressed={inspectorOpen}
              className={cn(
                "inline-flex size-7 shrink-0 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-foreground/[0.06] hover:text-foreground",
                inspectorOpen ? "bg-foreground/[0.09] text-foreground" : "",
              )}
            >
              <HugeiconsIcon icon={SparklesIcon} size={14} strokeWidth={1.75} />
            </button>
          </IconTooltip>
        ) : null}
    </div>
  );

  const todoSummary =
    !historyOpen && activeId ? <TodoSummaryChip sessionId={activeId} /> : null;

  const toggleWindowMaximize = () => {
    if (!hasTauriWindowMetadata()) return;
    void getCurrentWindow().toggleMaximize().catch(() => undefined);
  };

  if (variant === "sidebar") {
    return (
      <div className="altai-ai-topbar flex shrink-0 flex-col border-b border-border-subtle bg-card">
        <div className="flex h-10 min-w-0 items-center gap-1.5 px-2.5">
          {historyControl}
          <ChatTabStrip
            embedded
            openChatIds={openChatIds}
            onSelect={onSelectChat}
            onCloseChat={onCloseChat}
            onNewChat={onNewChat}
          />
          {onClose ? (
            <IconTooltip label="Close panel">
              <button
                type="button"
                onClick={onClose}
                aria-label="Close panel"
                className="inline-flex size-7 shrink-0 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-foreground/[0.06] hover:text-foreground"
              >
                <HugeiconsIcon icon={Cancel01Icon} size={13} strokeWidth={1.75} />
              </button>
            </IconTooltip>
          ) : null}
        </div>
        <div className="flex h-9 min-w-0 items-center gap-1.5 border-t border-border-subtle/70 px-2.5">
          {workspaceActions}
          <div className="min-w-0 flex-1" />
          {todoSummary}
        </div>
      </div>
    );
  }

  return (
    <div className="altai-ai-topbar flex shrink-0 flex-col border-b border-border-subtle bg-card">
      <div
        className={cn(
          "flex h-10 min-w-0 items-center gap-1.5 px-2.5",
          IS_MAC && "pl-20",
        )}
      >
        <div
          data-tauri-drag-region
          onDoubleClick={toggleWindowMaximize}
          className="h-full min-w-4 flex-1"
          aria-label="Window title bar"
        />
        {historyControl}
        {todoSummary}
        {workspaceActions}
        {onOpenSettings ? (
          <IconTooltip label="ALTAI Studio settings">
            <button
              type="button"
              onClick={onOpenSettings}
              aria-label="ALTAI Studio settings"
              className="inline-flex size-8 shrink-0 items-center justify-center rounded-lg text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
            >
              <HugeiconsIcon icon={Settings01Icon} size={14} strokeWidth={1.75} />
            </button>
          </IconTooltip>
        ) : null}
        {onOpenStudio ? (
          <button
            type="button"
            onClick={onOpenStudio}
            className="inline-flex h-8 shrink-0 items-center gap-1.5 rounded-lg border border-border bg-muted/45 px-2.5 text-[10.5px] font-medium text-foreground transition-colors hover:bg-accent"
          >
            <HugeiconsIcon icon={CodeIcon} size={13} strokeWidth={1.8} />
            <span className="hidden @[34rem]:inline">Open IDE</span>
          </button>
        ) : null}
        {USE_CUSTOM_WINDOW_CONTROLS ? (
          <>
            <span className="ml-1 h-5 w-px shrink-0 bg-border" />
            <WindowControls />
          </>
        ) : null}
      </div>
    </div>
  );
}

function RunInspector({ className, onClose }: { className?: string; onClose?: () => void }) {
  const [activityQuery, setActivityQuery] = useState("");
  const meta = useChatStore((s) => s.agentMeta);
  const sessionId = useChatStore((s) => s.activeSessionId);
  const planQueue = usePlanStore((s) => s.queue);
  const appliedPlanEdits = usePlanStore((s) => s.applied);
  const hydrateTodos = useTodosStore((s) => s.hydrate);
  const todos = useTodosStore((s) =>
    sessionId ? s.bySession[sessionId] ?? EMPTY_TODOS : EMPTY_TODOS,
  );
  const [checkpoints, setCheckpoints] = useState<CheckpointInfo[]>([]);

  useEffect(() => {
    if (sessionId) void hydrateTodos(sessionId);
  }, [hydrateTodos, sessionId]);

  useEffect(() => {
    let mounted = true;
    void native.checkpointList().then((items) => {
      if (mounted) setCheckpoints(items);
    });
    return () => {
      mounted = false;
    };
  }, [sessionId, planQueue.length]);

  const completedTodos = todos.filter((todo) => todo.status === "completed").length;
  const normalizedActivityQuery = activityQuery.trim().toLowerCase();
  const filteredActivity = meta.activity.filter((item) =>
    [item.label, item.detail, item.kind, item.tone]
      .filter(Boolean)
      .join("\n")
      .toLowerCase()
      .includes(normalizedActivityQuery),
  );
  const researchEvents = meta.activity.filter((item) => item.kind === "research");
  const mcpEvents = meta.activity.filter((item) => item.kind === "mcp");
  const tokenTotal = meta.tokens.inputTokens + meta.tokens.outputTokens;
  const running = meta.status === "thinking" || meta.status === "streaming";

  return (
    <aside
      aria-label="Run details"
      className={cn(
        "flex min-h-0 min-w-0 flex-col border-l border-border-subtle bg-card",
        className,
      )}
    >
      <SurfaceHeader
        title="Run details"
        eyebrow="Current run"
        icon={SparklesIcon}
        subtitle={
          meta.status === "idle" ? "Ready for the next task" : meta.step ?? "Agent is working"
        }
        status={
          <span
            className={cn(
              "rounded px-1.5 py-0.5 text-[8.5px] font-semibold",
              running
                ? "bg-primary/10 text-primary"
                : meta.error
                  ? "bg-destructive/10 text-destructive"
                  : "bg-muted text-muted-foreground",
            )}
          >
            {meta.error ? "Blocked" : running ? "Running" : "Idle"}
          </span>
        }
        onClose={onClose}
        actions={
          running ? (
            <button
              type="button"
              onClick={stopAgent}
              className="rounded-md border border-destructive/25 bg-destructive/[0.06] px-2 py-1 text-[9.5px] font-medium text-destructive hover:bg-destructive/10"
            >
              Stop run
            </button>
          ) : null
        }
      />

      <div className="min-h-0 flex-1 space-y-2.5 overflow-y-auto p-2.5">
        <section className="rounded-lg border border-border bg-muted/30 p-3">
          <div className="flex items-center gap-2">
            <AgentStatusPill announce={false} />
            <span className="ml-auto text-[9.5px] tabular-nums text-muted-foreground">
              {tokenTotal ? `${tokenTotal.toLocaleString()} tokens` : "No usage yet"}
            </span>
          </div>
          {meta.step ? (
            <p className="mt-2 line-clamp-2 text-[10.5px] leading-relaxed text-foreground">
              {meta.step}
            </p>
          ) : null}
          <div className="mt-3 grid grid-cols-2 gap-px overflow-hidden rounded-md border border-border bg-border">
            <InspectorMetric label="Plan" value={todos.length ? `${completedTodos}/${todos.length}` : "—"} />
            <InspectorMetric label="Changes" value={String(planQueue.length)} />
            <InspectorMetric label="Approvals" value={String(meta.pendingApprovals.length)} />
            <InspectorMetric label="Subagents" value={String(meta.activeSubagents.length)} />
          </div>
        </section>

        {meta.error ? (
          <section className="rounded-lg border border-destructive/30 bg-destructive/[0.06] p-3 text-[10.5px] leading-relaxed text-destructive">
            <div className="mb-1 text-[9px] font-semibold uppercase tracking-wide">Run blocked</div>
            {meta.error}
          </section>
        ) : null}

        {meta.pendingApprovals.length ? (
          <section>
            <div className="mb-1.5 px-1 text-[9px] font-semibold uppercase tracking-[0.12em] text-warning">
              Action required
            </div>
            <ApprovalsInspector approvals={meta.pendingApprovals} />
          </section>
        ) : null}

        <InspectorSection
          title="Plan"
          summary={
            todos.length
              ? `${completedTodos} of ${todos.length} steps complete`
              : "No checklist for this run"
          }
          count={todos.length}
          defaultOpen={todos.length > 0 && running}
        >
          <TodosInspector done={completedTodos} total={todos.length} todos={todos} />
        </InspectorSection>

        <InspectorSection
          title="Activity"
          summary="Chronological agent steps and tool results"
          count={meta.activity.length}
          defaultOpen
        >
          <SurfaceSearch
            value={activityQuery}
            onChange={setActivityQuery}
            placeholder="Filter activity"
            className="mb-2"
          />
          <ActivityInspector
            meta={meta}
            events={filteredActivity}
            hasQuery={Boolean(activityQuery.trim())}
            compact
          />
        </InspectorSection>

        <InspectorSection
          title="Changes & files"
          summary="Proposed edits and generated artifacts"
          count={planQueue.length + meta.artifacts.length}
          defaultOpen={planQueue.length > 0}
        >
          {planQueue.length ? (
            <ChangesInspector queue={planQueue} />
          ) : null}
          {planQueue.length && meta.artifacts.length ? (
            <div className="my-2 border-t border-border-subtle" />
          ) : null}
          {meta.artifacts.length ? (
            <ArtifactsInspector items={meta.artifacts} />
          ) : null}
          {!planQueue.length && !meta.artifacts.length ? (
            <InspectorEmpty>No changes or generated files yet.</InspectorEmpty>
          ) : null}
        </InspectorSection>

        <InspectorSection
          title="Research & tools"
          summary="External lookups and connected MCP calls"
          count={researchEvents.length + mcpEvents.length}
        >
          {researchEvents.length ? <ResearchInspector events={researchEvents} /> : null}
          {researchEvents.length && mcpEvents.length ? (
            <div className="my-2 border-t border-border-subtle" />
          ) : null}
          {mcpEvents.length ? <McpInspector events={mcpEvents} /> : null}
          {!researchEvents.length && !mcpEvents.length ? (
            <InspectorEmpty>No research or connected tool activity yet.</InspectorEmpty>
          ) : null}
        </InspectorSection>

        <InspectorSection
          title="Delegated work"
          summary="Subagents working on parts of this run"
          count={meta.activeSubagents.length}
        >
          <AgentsInspector tasks={meta.activeSubagents} />
        </InspectorSection>

        <InspectorSection
          title="Recovery"
          summary="Restore points created before agent edits"
          count={checkpoints.length + appliedPlanEdits.length}
        >
          <SnapshotsInspector
            items={checkpoints}
            applied={appliedPlanEdits}
            setItems={setCheckpoints}
          />
        </InspectorSection>
      </div>
    </aside>
  );
}

function InspectorSection({
  title,
  summary,
  count,
  defaultOpen = false,
  children,
}: {
  title: string;
  summary: string;
  count: number;
  defaultOpen?: boolean;
  children: React.ReactNode;
}) {
  const [open, setOpen] = useState(defaultOpen);
  return (
    <Collapsible
      open={open}
      onOpenChange={setOpen}
      className="overflow-hidden rounded-lg border border-border bg-card"
    >
      <CollapsibleTrigger className="group flex w-full items-center gap-2 px-3 py-2.5 text-left hover:bg-accent/60">
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-1.5">
            <span className="text-[10.5px] font-semibold text-foreground">{title}</span>
            {count ? (
              <span className="rounded bg-foreground/[0.06] px-1.5 text-[8.5px] tabular-nums text-muted-foreground">
                {count}
              </span>
            ) : null}
          </div>
          <div className="mt-0.5 truncate text-[9px] text-muted-foreground">{summary}</div>
        </div>
        <HugeiconsIcon
          icon={ArrowDown01Icon}
          size={11}
          strokeWidth={2}
          className={cn(
            "shrink-0 text-muted-foreground transition-transform",
            open && "rotate-180",
          )}
        />
      </CollapsibleTrigger>
      <CollapsibleContent className="border-t border-border-subtle bg-muted/10 p-2.5">
        {children}
      </CollapsibleContent>
    </Collapsible>
  );
}

function ActivityInspector({
  meta,
  events,
  hasQuery,
  compact = false,
}: {
  meta: ReturnType<typeof useChatStore.getState>["agentMeta"];
  events: ReturnType<typeof useChatStore.getState>["agentMeta"]["activity"];
  hasQuery: boolean;
  compact?: boolean;
}) {
  const tokenTotal = meta.tokens.inputTokens + meta.tokens.outputTokens;
  return (
    <div className="space-y-2">
      {!compact ? (
        <>
      <section className="rounded-md border border-border bg-muted/40 p-2.5">
        <div className="flex items-center gap-2">
          <AgentStatusPill announce={false} />
          <span className="ml-auto text-[10px] tabular-nums text-muted-foreground">
            {tokenTotal > 0 ? `${tokenTotal.toLocaleString()} tokens` : "No tokens yet"}
          </span>
        </div>
        {meta.step ? <p className="mt-2 text-[11px] leading-relaxed text-muted-foreground">{meta.step}</p> : null}
      </section>
      <section className="rounded-md border border-border bg-muted/30 p-2.5">
        <div className="text-[10px] font-medium uppercase tracking-wide text-muted-foreground">Run state</div>
        <div className="mt-2 grid grid-cols-2 gap-2 text-[11px]">
          <RunStateMetric label="Approvals" value={String(meta.approvalsPending)} />
          <RunStateMetric label="Subagents" value={String(meta.activeSubagents.length)} />
          <RunStateMetric label="Input" value={meta.tokens.inputTokens.toLocaleString()} />
          <RunStateMetric label="Output" value={meta.tokens.outputTokens.toLocaleString()} />
        </div>
      </section>
      {meta.error ? (
        <section className="border border-destructive/30 bg-destructive/[0.06] p-2.5 text-[11px] text-destructive">
          {meta.error}
        </section>
      ) : null}
        </>
      ) : null}
      <section
        className={cn(
          "rounded-md border border-border bg-muted/30 p-2.5",
          compact && "border-0 bg-transparent p-0",
        )}
      >
        <div className="text-[10px] font-medium uppercase tracking-wide text-muted-foreground">Timeline</div>
        {events.length ? (
          <div className="mt-2 space-y-2">
            {[...events].reverse().map((item) => (
              <div key={item.id} className="flex gap-2">
                <span
                  className={cn(
                    "mt-1.5 size-1.5 shrink-0 rounded-full",
                    item.tone === "success"
                      ? "bg-success"
                      : item.tone === "warning"
                        ? "bg-warning"
                        : item.tone === "error"
                          ? "bg-destructive"
                          : "bg-info",
                  )}
                />
                <div className="min-w-0 flex-1">
                  <div className="flex items-baseline gap-2">
                    <span className="min-w-0 flex-1 truncate text-[10.5px] text-foreground">{item.label}</span>
                    <time className="shrink-0 text-[9px] tabular-nums text-muted-foreground" dateTime={new Date(item.createdAt).toISOString()}>
                      {new Date(item.createdAt).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}
                    </time>
                  </div>
                  {item.detail ? <div className="mt-0.5 line-clamp-2 text-[9.5px] leading-relaxed text-muted-foreground">{item.detail}</div> : null}
                </div>
              </div>
            ))}
          </div>
        ) : (
          <p className="mt-2 text-[10.5px] leading-relaxed text-muted-foreground">
            {hasQuery
              ? "No timeline events match this search."
              : "Run events will appear here as the agent works."}
          </p>
        )}
      </section>
    </div>
  );
}

function ResearchInspector({
  events,
}: {
  events: ReturnType<typeof useChatStore.getState>["agentMeta"]["activity"];
}) {
  if (!events.length) {
    return <InspectorEmpty>Web searches, fetched pages, and paper lookups will appear here.</InspectorEmpty>;
  }
  return (
    <div className="space-y-2">
      {[...events].reverse().map((item) => (
        <div key={item.id} className="rounded-md border border-border bg-muted/30 px-2.5 py-2">
          <div className="flex items-center gap-2">
            <span className="size-1.5 shrink-0 rounded-full bg-info" />
            <span className="min-w-0 flex-1 truncate text-[11px] font-medium">{item.label}</span>
            <time className="text-[9px] tabular-nums text-muted-foreground" dateTime={new Date(item.createdAt).toISOString()}>
              {new Date(item.createdAt).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}
            </time>
          </div>
          {item.detail ? <div className="mt-1 pl-3.5 text-[10px] text-muted-foreground">{item.detail}</div> : null}
        </div>
      ))}
    </div>
  );
}

function McpInspector({
  events,
}: {
  events: ReturnType<typeof useChatStore.getState>["agentMeta"]["activity"];
}) {
  if (!events.length) {
    return <InspectorEmpty>MCP server calls will appear here when the agent uses a connected tool.</InspectorEmpty>;
  }
  return (
    <div className="space-y-2">
      {[...events].reverse().map((item) => (
        <div key={item.id} className="rounded-md border border-border bg-muted/30 px-2.5 py-2">
          <div className="flex items-center gap-2">
            <span className={cn("size-1.5 shrink-0 rounded-full", item.tone === "error" ? "bg-destructive" : item.tone === "success" ? "bg-success" : "bg-muted-foreground")} />
            <span className="min-w-0 flex-1 truncate text-[11px] font-medium">{item.label}</span>
            <time className="text-[9px] tabular-nums text-muted-foreground" dateTime={new Date(item.createdAt).toISOString()}>
              {new Date(item.createdAt).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}
            </time>
          </div>
          {item.detail ? <div className="mt-1 pl-3.5 text-[10px] text-muted-foreground">{item.detail}</div> : null}
        </div>
      ))}
    </div>
  );
}

function ArtifactsInspector({
  items,
}: {
  items: ReturnType<typeof useChatStore.getState>["agentMeta"]["artifacts"];
}) {
  if (!items.length) {
    return <InspectorEmpty>Files emitted by experiments and execution jobs will appear here.</InspectorEmpty>;
  }
  return (
    <div className="space-y-2">
      {[...items].reverse().map((item) => (
        <div key={item.id} className="flex items-center gap-2 rounded-md border border-border bg-muted/30 px-2.5 py-2">
          <HugeiconsIcon icon={FileEditIcon} size={12} strokeWidth={1.75} className="shrink-0 text-muted-foreground" />
          <div className="min-w-0 flex-1">
            <div className="truncate text-[11px] font-medium" title={item.path}>{item.path.split(/[\\/]/).pop() || item.path}</div>
            <div className="mt-0.5 truncate font-mono text-[9.5px] text-muted-foreground">{item.path}</div>
          </div>
          <button
            type="button"
            onClick={() => window.dispatchEvent(new CustomEvent<string>("altai:open-file", { detail: item.path }))}
            className="rounded-md bg-foreground/[0.07] px-1.5 py-1 text-[10px] font-medium text-foreground hover:bg-foreground/[0.12]"
          >
            Open
          </button>
        </div>
      ))}
    </div>
  );
}

function ChangesInspector({
  queue,
}: {
  queue: ReturnType<typeof usePlanStore.getState>["queue"];
}) {
  if (!queue.length) {
    return <InspectorEmpty>Planned and agent-made changes will appear here for review.</InspectorEmpty>;
  }
  return (
    <div className="space-y-2">
      <div className="rounded-md border border-border/50 bg-muted/20 p-2.5 text-[11px] leading-relaxed text-foreground">
        <div>
          {queue.length} proposed change{queue.length === 1 ? " is" : "s are"} waiting for review.
        </div>
        <button
          type="button"
          onClick={() =>
            window.dispatchEvent(new CustomEvent("altai:open-change-review"))
          }
          className="mt-2 rounded-md bg-foreground px-2 py-1 text-[10.5px] font-medium text-background"
        >
          Open change review
        </button>
      </div>
      {queue.map((change) => {
        const beforeLines = change.originalContent.split("\n").length;
        const afterLines = change.proposedContent.split("\n").length;
        const delta = afterLines - beforeLines;
        const name = change.path.split(/[/\\]/).pop() || change.path;
        return (
          <div key={change.id} className="rounded-md border border-border bg-muted/30 px-2.5 py-2">
            <div className="flex items-center gap-2">
              <HugeiconsIcon icon={FileEditIcon} size={12} strokeWidth={1.75} className="shrink-0 text-muted-foreground" />
              <span className="min-w-0 flex-1 truncate font-mono text-[10.5px] font-medium">{name}</span>
              {change.isNewFile ? <span className="text-[9.5px] text-success">new</span> : null}
              {!change.isNewFile ? (
                <span className={cn("text-[9.5px] tabular-nums", delta >= 0 ? "text-success" : "text-destructive")}>
                  {delta >= 0 ? "+" : ""}{delta}L
                </span>
              ) : null}
            </div>
            <div className="mt-1 truncate pl-5 font-mono text-[9.5px] text-muted-foreground">{change.path}</div>
          </div>
        );
      })}
    </div>
  );
}

function ApprovalsInspector({
  approvals,
}: {
  approvals: ReturnType<typeof useChatStore.getState>["agentMeta"]["pendingApprovals"];
}) {
  const respond = useChatStore((s) => s.respondToApproval);
  if (!approvals.length) {
    return <InspectorEmpty>Actions that need your approval will appear here without interrupting the task view.</InspectorEmpty>;
  }
  return (
    <div className="space-y-2">
      {approvals.map((approval) => (
        <div key={approval.id} className="rounded-md border border-warning/30 bg-warning/[0.06] p-2.5">
          <div className="flex items-center gap-2">
            <span className="size-1.5 animate-pulse rounded-full bg-warning" />
            <span className="min-w-0 flex-1 truncate text-[11px] font-medium">{approval.action}</span>
          </div>
          <pre className="mt-2 max-h-24 max-w-full min-w-0 overflow-x-auto whitespace-pre-wrap break-words rounded-md bg-muted p-2 font-mono text-[9.5px] leading-relaxed text-muted-foreground [overflow-wrap:anywhere]">
            {approvalPreview(approval.payload)}
          </pre>
          <div className="mt-2 flex justify-end gap-1.5">
            <button type="button" onClick={() => respond(approval.id, false)} className="rounded-md px-2 py-1 text-[10px] font-medium text-muted-foreground hover:bg-muted hover:text-foreground">Deny</button>
            <button type="button" onClick={() => respond(approval.id, true)} className="rounded-md bg-foreground px-2 py-1 text-[10px] font-medium text-background hover:bg-foreground/90">Approve</button>
          </div>
        </div>
      ))}
    </div>
  );
}

function approvalPreview(payload: unknown): string {
  try {
    const serialized = JSON.stringify(payload, null, 2) ?? String(payload);
    return serialized.length > 900 ? `${serialized.slice(0, 900)}…` : serialized;
  } catch {
    return String(payload);
  }
}

function SnapshotsInspector({
  items,
  applied,
  setItems,
}: {
  items: CheckpointInfo[];
  applied: AppliedPlanEdit[];
  setItems: (items: CheckpointInfo[]) => void;
}) {
  const [restoring, setRestoring] = useState<string | null>(null);
  const restoreApplied = usePlanStore((s) => s.restoreApplied);
  const [error, setError] = useState<string | null>(null);
  if (!items.length && !applied.length) return <InspectorEmpty>Before-agent-edit and reviewed-change snapshots will appear here, ready to restore safely.</InspectorEmpty>;
  const restore = async (id: string) => {
    if (restoring) return;
    setError(null);
    setRestoring(id);
    try {
      await native.checkpointRestore(id);
      setItems(await native.checkpointList());
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setRestoring(null);
    }
  };
  const restorePlan = async (id: string) => {
    if (restoring) return;
    setError(null);
    setRestoring(id);
    try {
      const result = await restoreApplied(id);
      if (result && !result.ok) setError(result.error ?? "Could not restore change.");
    } finally {
      setRestoring(null);
    }
  };
  return (
    <div className="space-y-2">
      {applied.length ? (
        <section className="space-y-2">
          <div className="px-1 text-[10px] font-medium uppercase tracking-wide text-muted-foreground">Plan review</div>
          {[...applied].reverse().map((item) => (
            <div key={item.id} className="flex items-center gap-2 rounded-md border border-border bg-muted/30 px-2.5 py-2">
              <HugeiconsIcon icon={FileEditIcon} size={12} strokeWidth={1.75} className="shrink-0 text-muted-foreground" />
              <div className="min-w-0 flex-1">
                <div className="truncate text-[11px] font-medium" title={item.path}>{item.path.split(/[\\/]/).pop()}</div>
                <div className="mt-0.5 text-[9.5px] text-muted-foreground">Accepted change · {item.isNewFile ? "removes new file" : "restores prior content"}</div>
              </div>
              <button type="button" disabled={restoring === item.id} onClick={() => void restorePlan(item.id)} className="rounded-md bg-foreground/[0.07] px-1.5 py-1 text-[10px] font-medium text-foreground hover:bg-foreground/[0.12] disabled:opacity-50">
                {restoring === item.id ? "…" : "Restore"}
              </button>
            </div>
          ))}
        </section>
      ) : null}
      {items.length ? <div className="px-1 pt-1 text-[10px] font-medium uppercase tracking-wide text-muted-foreground">Agent edits</div> : null}
      {items.map((item) => (
        <div key={item.id} className="flex items-center gap-2 rounded-md border border-border bg-muted/30 px-2.5 py-2">
          <HugeiconsIcon icon={FileEditIcon} size={12} strokeWidth={1.75} className="shrink-0 text-muted-foreground" />
          <div className="min-w-0 flex-1">
            <div className="truncate text-[11px] font-medium" title={item.path}>{item.path.split(/[\\/]/).pop()}</div>
            <div className="mt-0.5 text-[9.5px] text-muted-foreground">{item.label}</div>
          </div>
          <button type="button" disabled={restoring === item.id} onClick={() => void restore(item.id)} className="rounded-md bg-foreground/[0.07] px-1.5 py-1 text-[10px] font-medium text-foreground hover:bg-foreground/[0.12] disabled:opacity-50">
            {restoring === item.id ? "…" : "Restore"}
          </button>
        </div>
      ))}
      {error ? <div className="border border-destructive/30 bg-destructive/[0.06] p-2 text-[10.5px] text-destructive">{error}</div> : null}
    </div>
  );
}

function Body({
  hasComposer,
  onOpenReview,
  projectTarget,
}: {
  hasComposer: boolean;
  onOpenReview: () => void;
  projectTarget?: {
    name: string;
    path: string | null;
    kind: "local" | "github" | null;
    onChange: () => void;
  };
}) {
  const nativeMessages = useChatStore((s) => s.nativeMessages);
  const agentStatus = useChatStore((s) => s.agentMeta.status);
  const errorText = useChatStore((s) => s.agentMeta.error);
  const respondToApproval = useChatStore((s) => s.respondToApproval);
  const patchAgentMeta = useChatStore((s) => s.patchAgentMeta);

  const displayMessages = nativeMessages;
  const displayStatus =
    agentStatus === "streaming" || agentStatus === "thinking"
      ? "streaming"
      : "ready";

  return (
    <div
      id="altai-active-chat"
      role="tabpanel"
      aria-label="Active chat session"
      tabIndex={-1}
      className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden"
    >
      <PlanModeStrip />

      <div className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
        {displayMessages.length === 0 ? (
          <EmptyState />
        ) : (
          <div className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden [&_.text-sm]:text-[12.5px] [&_p]:leading-relaxed">
            <AiChatView
              messages={displayMessages}
              status={displayStatus}
              error={errorText ? new Error(errorText) : undefined}
              clearError={() => patchAgentMeta({ error: null })}
              addToolApprovalResponse={({ id, approved }) =>
                respondToApproval(id, approved)
              }
              stop={stopAgent}
            />
          </div>
        )}
      </div>

      <RunRecoveryActions />
      <ClarificationChoices />
      <ChangeReviewBanner onOpen={onOpenReview} />
      {hasComposer ? (
        <AiInputBar />
      ) : (
        <AiInputBarConnect onAdd={() => void openSettingsWindow("models")} />
      )}
      {projectTarget ? <ChatProjectTarget {...projectTarget} /> : null}
    </div>
  );
}

function ChatProjectTarget({
  name,
  path,
  kind,
  onChange,
}: {
  name: string;
  path: string | null;
  kind: "local" | "github" | null;
  onChange: () => void;
}) {
  const label = path ? name : "Choose a project";
  const detail =
    kind === "github"
      ? "GitHub repository"
      : path
        ? "Local folder"
        : "Optional · Local folder or GitHub";

  return (
    <div className="flex min-w-0 shrink-0 px-3 pb-2 pt-1">
      <button
        type="button"
        onClick={onChange}
        className="group flex h-8 min-w-0 max-w-full items-center gap-2 rounded-lg border border-border/70 bg-muted/25 px-2.5 text-left text-muted-foreground transition-colors hover:border-border hover:bg-accent hover:text-foreground"
        aria-label={path ? `Change project, currently ${name}` : "Choose a project"}
      >
        <HugeiconsIcon
          icon={kind === "github" ? GithubIcon : Folder01Icon}
          size={13}
          strokeWidth={1.75}
          className="shrink-0"
        />
        <span className="min-w-0 truncate text-[10.5px] font-medium text-foreground">
          {label}
        </span>
        <span className="hidden shrink-0 text-[9.5px] text-muted-foreground @[28rem]:inline">
          {detail}
        </span>
        <HugeiconsIcon
          icon={ArrowDown01Icon}
          size={11}
          strokeWidth={2}
          className="shrink-0 text-muted-foreground/70"
        />
      </button>
    </div>
  );
}

function RunRecoveryActions() {
  const sessionId = useChatStore((s) => s.activeSessionId);
  const focusInput = useChatStore((s) => s.focusInput);
  const run = useAgentRunsStore((s) =>
    sessionId ? s.runs[sessionId] : undefined,
  );
  const [submitting, setSubmitting] = useState(false);

  if (!run?.runId) return null;
  const warning = !run.completed ? run.warning : null;
  const outcome = run.completed ? run.outcome : null;
  const canContinue =
    outcome?.kind === "stuck" || outcome?.kind === "budget_exhausted";
  const canRetry = isRetryableRunOutcome(outcome);
  if (!warning && !canContinue && !canRetry) return null;

  const detail = warning
    ? `${describeRunWarning(warning)}. You can steer, stop, or dismiss — the run is still working.`
    : outcome?.kind === "stuck"
      ? `The run paused because it was ${outcome.reason.replace(/_/g, " ")}.`
      : outcome?.kind === "budget_exhausted"
        ? `Hit the turn limit after ${outcome.budget.iterations_used} steps. Continue picks up where it left off.`
        : "The provider request failed after its retry policy was exhausted.";

  const dismissWarning = () => {
    dismissRunAttention(sessionId);
  };

  const continueRun = async () => {
    if (submitting) return;
    setSubmitting(true);
    dismissWarning();
    try {
      await sendMessage(
        outcome?.kind === "budget_exhausted"
          ? continueBudgetSegmentPrompt()
          : continueStuckPrompt(),
      );
    } finally {
      setSubmitting(false);
    }
  };

  const retryRun = async () => {
    if (submitting) return;
    setSubmitting(true);
    dismissWarning();
    try {
      await retryFailedRun();
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div
      role={warning ? "status" : "alert"}
      className="mx-3 mb-2 rounded-lg border border-warning/35 bg-warning/[0.08] px-3 py-2.5"
    >
      <div className="text-[11px] font-medium text-foreground">
        {warning
          ? "Possible repeated failure"
          : canRetry
            ? "Retry available"
            : outcome?.kind === "budget_exhausted"
              ? "Turn limit reached"
              : "Run paused"}
      </div>
      <div className="mt-0.5 text-[10.5px] leading-relaxed text-muted-foreground">
        {detail}
      </div>
      <div className="mt-2 flex flex-wrap gap-1.5">
        {canContinue ? (
          <button
            type="button"
            disabled={submitting}
            onClick={() => void continueRun()}
            className="rounded-md bg-foreground px-2 py-1 text-[10.5px] font-medium text-background disabled:opacity-50"
          >
            Continue
          </button>
        ) : null}
        {canRetry ? (
          <button
            type="button"
            disabled={submitting}
            onClick={() => void retryRun()}
            className="rounded-md bg-foreground px-2 py-1 text-[10.5px] font-medium text-background disabled:opacity-50"
          >
            Retry
          </button>
        ) : null}
        {warning || canContinue ? (
          <button
            type="button"
            onClick={() => {
              dismissWarning();
              focusInput(
                warning
                  ? "Adjust the active run with this direction: "
                  : "Continue the previous run with this adjustment: ",
              );
            }}
            className="rounded-md border border-border bg-muted px-2 py-1 text-[10.5px] font-medium text-foreground hover:bg-accent"
          >
            Steer
          </button>
        ) : null}
        {warning ? (
          <button
            type="button"
            onClick={() => {
              dismissWarning();
              stopAgent();
            }}
            className="rounded-md border border-border bg-muted px-2 py-1 text-[10.5px] font-medium text-foreground hover:bg-accent"
          >
            Stop
          </button>
        ) : null}
        {warning ? (
          <button
            type="button"
            onClick={dismissWarning}
            className="rounded-md border border-border bg-muted px-2 py-1 text-[10.5px] font-medium text-foreground hover:bg-accent"
          >
            Dismiss
          </button>
        ) : null}
      </div>
    </div>
  );
}

function ClarificationChoices() {
  const choices = useChatStore((s) => s.pendingChoices);
  const editDiff = useChatStore((s) => s.pendingEditDiff);

  // A file-edit approval (from the crate's edit gate) takes precedence over
  // the plain choice chips: it renders a richer diff-review card with
  // Approve / Deny actions. The reply still rides the clarification channel.
  if (editDiff) {
    return (
      <EditApprovalCard
        diff={editDiff}
        onRespond={(choice) => void sendMessage(choice)}
      />
    );
  }

  if (!choices || choices.length === 0) return null;
  return (
    <div
      role="group"
      aria-label="Suggested replies"
      className="flex shrink-0 flex-wrap gap-1.5 border-t border-border-subtle px-3 py-2"
    >
      <span aria-live="polite" className="sr-only">
        {choices.length} suggested{" "}
        {choices.length === 1 ? "reply" : "replies"} available
      </span>
      {choices.map((choice, i) => (
        <button
          key={`${i}-${choice}`}
          type="button"
          onClick={() => void sendMessage(choice)}
          className="rounded-md border border-border bg-muted px-3 py-1 text-[11px] font-medium text-foreground transition-colors hover:bg-accent"
        >
          {choice}
        </button>
      ))}
    </div>
  );
}

function ChangeReviewBanner({ onOpen }: { onOpen: () => void }) {
  const queueLen = usePlanStore((s) => s.queue.length);
  if (queueLen === 0) return null;
  return (
    <div className="altai-ai-review-banner mx-3 mb-2 flex shrink-0 items-center gap-2.5 rounded-lg border border-primary/20 bg-primary/[0.055] px-3 py-2">
      <span className="flex size-7 shrink-0 items-center justify-center rounded-md bg-primary/10 text-primary">
        <HugeiconsIcon icon={FileEditIcon} size={13} strokeWidth={1.8} />
      </span>
      <span className="min-w-0 flex-1">
        <span className="block text-[11px] font-medium text-foreground">Changes ready</span>
        <span className="block truncate text-[10px] text-muted-foreground">
          {queueLen} proposed change{queueLen === 1 ? "" : "s"} waiting for review
        </span>
      </span>
      <button
        type="button"
        onClick={onOpen}
        className="rounded-md bg-primary px-2.5 py-1.5 text-[10.5px] font-medium text-primary-foreground transition-colors hover:bg-primary/90"
      >
        Review changes
      </button>
    </div>
  );
}

function PlanModeStrip() {
  const active = usePlanStore((s) => s.active);
  const queueLen = usePlanStore((s) => s.queue.length);
  const disable = usePlanStore((s) => s.disable);
  if (!active) return null;
  return (
    <div className="flex shrink-0 items-center gap-2 border-b border-border-subtle bg-warning/[0.035] px-3 py-1.5">
      <span className="size-1.5 shrink-0 rounded-full bg-warning" />
      <span className="text-[11px] font-medium text-foreground">Plan mode</span>
      <span className="text-[11px] text-muted-foreground">
        {queueLen > 0 ? `· ${queueLen} queued` : "· no edits queued"}
      </span>
      <span className="flex-1" />
      {queueLen > 0 ? (
        <button
          type="button"
          onClick={() =>
            window.dispatchEvent(new CustomEvent("altai:open-change-review"))
          }
          className="rounded-md px-1.5 py-0.5 text-[10.5px] font-medium text-foreground transition-colors hover:bg-foreground/[0.06]"
        >
          Review
        </button>
      ) : null}
      <button
        type="button"
        onClick={() => disable()}
        className="rounded-md px-1.5 py-0.5 text-[10.5px] text-muted-foreground transition-colors hover:bg-foreground/[0.06] hover:text-foreground"
      >
        Exit
      </button>
    </div>
  );
}

function EmptyState() {
  const activeId = useAgentsStore((s) => s.activeId);
  const customAgents = useAgentsStore((s) => s.customAgents);
  void customAgents;

  const agents = useAgentsStore.getState().all();
  const active = agents.find((a) => a.id === activeId) ?? agents[0];

  return (
    <div className="altai-ai-task-home flex min-h-0 flex-1 flex-col overflow-y-auto px-4 py-5 @[36rem]:px-6 @[36rem]:py-7">
      <div className="mx-auto flex w-full max-w-[32rem] flex-1 flex-col justify-center">
        <div className="altai-ai-task-header">
          <div className="flex size-9 shrink-0 items-center justify-center rounded-xl border border-primary/20 bg-primary/10 text-primary">
            <HugeiconsIcon icon={SparklesIcon} size={17} strokeWidth={1.75} />
          </div>
          <div className="min-w-0">
            <div className="text-[10px] font-medium uppercase tracking-[0.13em] text-muted-foreground">
              {active.name} · ready
            </div>
            <h2 className="mt-1.5 text-[20px] font-semibold tracking-tight text-foreground">
              Start with the outcome
            </h2>
            <p className="mt-1 max-w-[31rem] text-[11.5px] leading-relaxed text-muted-foreground">
              Describe what should change and how we will know it is done. ALTAI can inspect context, work across files, and verify the result.
            </p>
          </div>
        </div>
      </div>

      <div className="flex shrink-0 items-center justify-center gap-1.5 pt-4 text-[10px] text-muted-foreground/70">
        <span>Files, terminal, and previews stay available from Open IDE.</span>
      </div>
    </div>
  );
}
