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
import { Dialog, DialogContent } from "@/components/ui/dialog";
import {
  Cancel01Icon,
  Clock01Icon,
  CodeIcon,
  Settings01Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { type ReactElement, useEffect, useRef, useState } from "react";
import {
  ActivityInspector,
  AgentsInspector,
  ApprovalsInspector,
  ArtifactsInspector,
  ChangeReviewBanner,
  ChangesInspector,
  ChatProjectTarget,
  ChatTabStrip,
  ClarificationChoices,
  EmptyState,
  InspectorEmpty,
  InspectorSection,
  McpInspector,
  PlanModeStrip,
  ResearchInspector,
  RunActionRequiredSection,
  RunBlockedBanner,
  RunDetailsHeader,
  RunOverviewCard,
  RunRecoveryActions,
  SnapshotsInspector,
  SurfaceSearch,
  TodosInspector,
  WorkspaceTargetForm,
  WorkspaceTopbarActions,
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
        <WorkspaceTargetForm
          busy={busy}
          error={error}
          repoUrl={repoUrl}
          onRepoUrlChange={setRepoUrl}
          canChooseLocal={Boolean(onChooseLocalWorkspace)}
          canCloneGithub={Boolean(onCloneGithubRepository)}
          showClearProject={Boolean(workspacePath && onClearWorkspace)}
          onChooseLocal={() => void chooseLocal()}
          onCloneGithub={() => void cloneGithub()}
          onClearProject={() => {
            onClearWorkspace?.();
            onOpenChange(false);
          }}
        />
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
function ChatTabStripBridge({
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
  const tabs = openChatIds
    .map((id) => sessions.find((session) => session.id === id))
    .filter((session): session is NonNullable<typeof session> => Boolean(session))
    .map((session) => ({ id: session.id, title: session.title }));

  return (
    <ChatTabStrip
      tabs={tabs}
      activeId={activeId}
      embedded={embedded}
      onSelect={(id) => {
        switchSession(id);
        onSelect();
      }}
      onClose={onCloseChat}
      onNewChat={onNewChat}
      renderTooltip={(label, children) => (
        <IconTooltip label={label}>{children}</IconTooltip>
      )}
    />
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
    <WorkspaceTopbarActions
      variant={variant}
      workOpen={workOpen}
      inboxOpen={inboxOpen}
      inboxAttentionCount={inboxAttentionCount}
      inspectorOpen={inspectorOpen}
      inspectorAvailable={inspectorAvailable}
      onToggleWork={onToggleWork}
      onToggleInbox={onToggleInbox}
      onToggleInspector={onToggleInspector}
      renderTooltip={(label, children) => (
        <IconTooltip label={label}>{children}</IconTooltip>
      )}
    />
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
          <ChatTabStripBridge
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
  const respondToApproval = useChatStore((s) => s.respondToApproval);
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
      <RunDetailsHeader
        subtitle={
          meta.status === "idle" ? "Ready for the next task" : meta.step ?? "Agent is working"
        }
        status={meta.error ? "blocked" : running ? "running" : "idle"}
        onClose={onClose}
        onStop={stopAgent}
      />

      <div className="min-h-0 flex-1 space-y-2.5 overflow-y-auto p-2.5">
        <RunOverviewCard
          statusPill={<AgentStatusPill announce={false} />}
          tokenLabel={
            tokenTotal ? `${tokenTotal.toLocaleString()} tokens` : "No usage yet"
          }
          step={meta.step}
          metrics={[
            {
              label: "Plan",
              value: todos.length ? `${completedTodos}/${todos.length}` : "—",
            },
            { label: "Changes", value: String(planQueue.length) },
            {
              label: "Approvals",
              value: String(meta.pendingApprovals.length),
            },
            {
              label: "Subagents",
              value: String(meta.activeSubagents.length),
            },
          ]}
        />

        {meta.error ? <RunBlockedBanner message={meta.error} /> : null}

        {meta.pendingApprovals.length ? (
          <RunActionRequiredSection>
            <ApprovalsInspector
              approvals={meta.pendingApprovals}
              onRespond={respondToApproval}
            />
          </RunActionRequiredSection>
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
            events={filteredActivity}
            hasQuery={Boolean(activityQuery.trim())}
            compact
            step={meta.step}
            error={meta.error}
            approvalsPending={meta.approvalsPending}
            subagentCount={meta.activeSubagents.length}
            inputTokens={meta.tokens.inputTokens}
            outputTokens={meta.tokens.outputTokens}
            statusPill={<AgentStatusPill announce={false} />}
          />
        </InspectorSection>

        <InspectorSection
          title="Changes & files"
          summary="Proposed edits and generated artifacts"
          count={planQueue.length + meta.artifacts.length}
          defaultOpen={planQueue.length > 0}
        >
          {planQueue.length ? (
            <ChangesInspector
              queue={planQueue}
              onOpenReview={() =>
                window.dispatchEvent(new CustomEvent("altai:open-change-review"))
              }
            />
          ) : null}
          {planQueue.length && meta.artifacts.length ? (
            <div className="my-2 border-t border-border-subtle" />
          ) : null}
          {meta.artifacts.length ? (
            <ArtifactsInspector
              items={meta.artifacts}
              onOpenFile={(path) =>
                window.dispatchEvent(
                  new CustomEvent<string>("altai:open-file", { detail: path }),
                )
              }
            />
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
          <SnapshotsInspectorBridge
            items={checkpoints}
            applied={appliedPlanEdits}
            setItems={setCheckpoints}
          />
        </InspectorSection>
      </div>
    </aside>
  );
}

function SnapshotsInspectorBridge({
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

  return (
    <SnapshotsInspector
      items={items}
      applied={applied}
      restoringId={restoring}
      error={error}
      onRestoreCheckpoint={async (id) => {
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
      }}
      onRestoreApplied={async (id) => {
        if (restoring) return;
        setError(null);
        setRestoring(id);
        try {
          const result = await restoreApplied(id);
          if (result && !result.ok) {
            setError(result.error ?? "Could not restore change.");
          }
        } finally {
          setRestoring(null);
        }
      }}
    />
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
  const reviewQueueLen = usePlanStore((s) => s.queue.length);
  const planModeActive = usePlanStore((s) => s.active);
  const disablePlanMode = usePlanStore((s) => s.disable);
  const activeAgentId = useAgentsStore((s) => s.activeId);
  const customAgents = useAgentsStore((s) => s.customAgents);
  void customAgents;
  const agents = useAgentsStore.getState().all();
  const activeAgentName =
    agents.find((a) => a.id === activeAgentId)?.name ??
    agents[0]?.name ??
    "ALTAI";

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
      <PlanModeStrip
        active={planModeActive}
        queueLen={reviewQueueLen}
        onReview={() =>
          window.dispatchEvent(new CustomEvent("altai:open-change-review"))
        }
        onExit={() => disablePlanMode()}
      />

      <div className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
        {displayMessages.length === 0 ? (
          <EmptyState agentName={activeAgentName} />
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

      <RunRecoveryActionsBridge />
      <ClarificationChoicesBridge />
      <ChangeReviewBanner
        queueLen={reviewQueueLen}
        onOpen={onOpenReview}
      />
      {hasComposer ? (
        <AiInputBar />
      ) : (
        <AiInputBarConnect onAdd={() => void openSettingsWindow("models")} />
      )}
      {projectTarget ? <ChatProjectTarget {...projectTarget} /> : null}
    </div>
  );
}

function RunRecoveryActionsBridge() {
  const sessionId = useChatStore((s) => s.activeSessionId);
  const focusInput = useChatStore((s) => s.focusInput);
  const run = useAgentRunsStore((s) =>
    sessionId ? s.runs[sessionId] : undefined,
  );

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

  const title = warning
    ? "Possible repeated failure"
    : canRetry
      ? "Retry available"
      : outcome?.kind === "budget_exhausted"
        ? "Turn limit reached"
        : "Run paused";

  const dismissWarning = () => {
    dismissRunAttention(sessionId);
  };

  return (
    <RunRecoveryActions
      warning={Boolean(warning)}
      title={title}
      detail={detail}
      canContinue={canContinue}
      canRetry={canRetry}
      onContinue={async () => {
        dismissWarning();
        await sendMessage(
          outcome?.kind === "budget_exhausted"
            ? continueBudgetSegmentPrompt()
            : continueStuckPrompt(),
        );
      }}
      onRetry={async () => {
        dismissWarning();
        await retryFailedRun();
      }}
      onSteer={() => {
        dismissWarning();
        focusInput(
          warning
            ? "Adjust the active run with this direction: "
            : "Continue the previous run with this adjustment: ",
        );
      }}
      onStop={() => {
        dismissWarning();
        stopAgent();
      }}
      onDismiss={dismissWarning}
    />
  );
}

function ClarificationChoicesBridge() {
  const choices = useChatStore((s) => s.pendingChoices);
  const editDiff = useChatStore((s) => s.pendingEditDiff);
  return (
    <ClarificationChoices
      choices={choices}
      editDiff={editDiff}
      onRespond={(choice) => void sendMessage(choice)}
    />
  );
}

