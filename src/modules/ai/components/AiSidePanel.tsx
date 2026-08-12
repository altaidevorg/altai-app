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
  Clock01Icon,
  CodeIcon,
  Settings01Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  type ReactElement,
  useEffect,
  useRef,
  useState,
} from "react";
import {
  ActivityInspector,
  AgentsInspector,
  AiChatMainColumn,
  AiPanelTopbar,
  AiRunInspectorFrame,
  AiSidePanelFrame,
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
  resolveSidePanelChromeLayout,
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
  HISTORY_PANEL_MAX_WIDTH,
  HISTORY_PANEL_MIN_WIDTH,
  HISTORY_PANEL_WIDTH_KEY,
  INSPECTOR_PANEL_MAX_WIDTH,
  INSPECTOR_PANEL_MIN_WIDTH,
  INSPECTOR_PANEL_WIDTH_KEY,
  readPanelWidthFromStorage,
  writePanelWidthToStorage,
  toggleSidePanelChromeSurface,
  reconcileOpenChatTabIds,
  resolveSidePanelOpenEvent,
  closeChatTabSelection,
  openIdsAfterNewChat,
  type SidePanelChromeSurface,
  buildOperationsOpenIntent,
  isTextEditingKeyboardTarget,
  shouldDismissSidePanelOnEscape,
  chatTabsFromOpenIds,
  countCompletedTodos,
  filterActivityByQuery,
  filterActivityByKind,
  isAgentRunBusy,
  sumRunTokens,
  sessionIds,
  runInspectorHeaderSubtitle,
  runInspectorUsageTokenLabel,
  planProgressMetricValue,
  planInspectorSectionSummary,
  isRecoverableRunOutcome,
  runRecoveryPresentation,
  runRecoverySteerPrompt,
  historyToggleLabel,
  SIDE_PANEL_SETTINGS_LABEL,
  SIDE_PANEL_WINDOW_TITLE_BAR_LABEL,
  SIDE_PANEL_CHAT_SESSIONS_ARIA,
  SIDE_PANEL_RESIZE_HISTORY_LABEL,
  SIDE_PANEL_RESIZE_INSPECTOR_LABEL,
  SIDE_PANEL_RUN_DETAILS_ARIA,
  SIDE_PANEL_LOCAL_WORKSPACE_FALLBACK,
  INSPECTOR_ACTIVITY_TITLE,
  INSPECTOR_ACTIVITY_SUMMARY,
  INSPECTOR_ACTIVITY_FILTER_PLACEHOLDER,
  INSPECTOR_CHANGES_TITLE,
  INSPECTOR_CHANGES_SUMMARY,
  INSPECTOR_RESEARCH_TITLE,
  INSPECTOR_RESEARCH_SUMMARY,
  INSPECTOR_DELEGATED_TITLE,
  INSPECTOR_DELEGATED_SUMMARY,
  INSPECTOR_RECOVERY_TITLE,
  INSPECTOR_RECOVERY_SUMMARY,
  INSPECTOR_METRIC_APPROVALS,
  INSPECTOR_METRIC_SUBAGENTS,
  INSPECTOR_METRIC_CHANGES,
  INSPECTOR_METRIC_PLAN,
  PLAN_RESTORE_FALLBACK_ERROR,
} from "@altai/agent-ui";
import {
  retryFailedRun,
  sendMessage,
  stop as stopAgent,
  useChatStore,
} from "../store/chatStore";
import { useAgentRunsStore, type RunState } from "../store/agentRunsStore";
import {
  continueBudgetSegmentPrompt,
  continueStuckPrompt,
  describeTerminalOutcomeAttention,
  describeRunWarning,
  dismissRunAttention,
  hydratePersistedAgentRun,
  isRetryableRunOutcome,
  resolveAgentRunDeepLink,
  type AgentRunDeepLink,
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
import { TodoSummaryChip } from "./TodoStrip";

// Zustand selectors must return a stable reference when a session has no
// todos yet; allocating `[]` inside the selector triggers React's external
// store loop detector and can blank the whole renderer.
const EMPTY_TODOS: Array<{ id: string; title: string; status: string }> = [];

type PersistedRunSelection = AgentRunDeepLink & {
  snapshot: RunState | null;
  loading: boolean;
  error: string | null;
};

function readPanelWidth(
  key: string,
  fallback: number,
  min: number,
  max: number,
): number {
  return readPanelWidthFromStorage(window.localStorage, key, fallback, min, max);
}

function persistPanelWidth(key: string, width: number, min: number, max: number) {
  writePanelWidthToStorage(window.localStorage, key, width, min, max);
}

/** Canonical Work / Inbox destinations live under Operations, not AI overlays. */
function openOperationsSurface(
  view: "work" | "inbox" | "runs" | "overview",
  workHubView?: "runs" | "scheduled",
): void {
  if (typeof window === "undefined") return;
  window.dispatchEvent(
    new CustomEvent("altai:open-operations", {
      detail: buildOperationsOpenIntent(view, workHubView),
    }),
  );
}

export type AiSidePanelProps = {
  onClose?: () => void;
  hasComposer?: boolean;
  variant?: "workspace" | "sidebar";
  showTopbar?: boolean;
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
  showTopbar = true,
  workspaceName = SIDE_PANEL_LOCAL_WORKSPACE_FALLBACK,
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
      const target = e.target as HTMLElement | null;
      // Don't compete with Radix popovers/menus/dialogs — their own
      // dismiss handlers should run first. Radix sets data-state="open"
      // on triggers and renders portaled overlays with role="menu" /
      // role="listbox" / role="dialog".
      const hasOpenOverlay =
        !!target?.closest('[data-state="open"]') ||
        !!document.querySelector(
          '[role="menu"][data-state="open"], [role="listbox"][data-state="open"], [role="dialog"][data-state="open"]',
        );
      if (
        !shouldDismissSidePanelOnEscape({
          key: e.key,
          metaKey: e.metaKey,
          ctrlKey: e.ctrlKey,
          altKey: e.altKey,
          isEditableTarget: isTextEditingKeyboardTarget({
            tagName: target?.tagName,
            isContentEditable: target?.isContentEditable,
          }),
          hasOpenOverlay,
        })
      ) {
        return;
      }
      onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  const [activeSurface, setActiveSurface] = useState<SidePanelChromeSurface>(null);
  const [persistedRun, setPersistedRun] = useState<PersistedRunSelection | null>(
    null,
  );
  const persistedRunGeneration = useRef(0);
  const [openChatIds, setOpenChatIds] = useState<string[]>([]);
  const [reviewOpen, setReviewOpen] = useState(false);
  const [targetDialogOpen, setTargetDialogOpen] = useState(false);
  const panelRootRef = useRef<HTMLElement | null>(null);
  const [panelWidth, setPanelWidth] = useState(0);
  const historyOpen = activeSurface === "history";
  const inspectorOpen = activeSurface === "inspector";
  // Breakpoints live in agent-ui so VS Code and Desktop share density decisions.
  const {
    inspectorAvailable,
    showHistorySidebar,
    showInspectorSidebar,
  } = resolveSidePanelChromeLayout({
    variant: variant === "workspace" ? "workspace" : "sidebar",
    panelWidth,
    inspectorOpen,
    hasSession: Boolean(sessionId),
  });
  const toggleSurface = (surface: Exclude<SidePanelChromeSurface, null>) => {
    persistedRunGeneration.current += 1;
    setPersistedRun(null);
    setReviewOpen(false);
    setActiveSurface((current) => toggleSidePanelChromeSurface(current, surface));
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
        view?: "runs" | "scheduled";
      }>).detail;
      const resolved = resolveSidePanelOpenEvent(detail);
      if (resolved.kind === "review") {
        setActiveSurface(null);
        setReviewOpen(true);
        return;
      }
      if (resolved.kind === "surface") {
        setReviewOpen(false);
        setActiveSurface(resolved.surface);
        return;
      }
      if (resolved.kind === "operations") {
        setActiveSurface(null);
        openOperationsSurface(resolved.view, resolved.workHubView);
      }
    };
    window.addEventListener("altai:open-ai-surface", openSurface);
    return () => window.removeEventListener("altai:open-ai-surface", openSurface);
  }, []);

  useEffect(() => {
    const openAgentRun = (event: Event) => {
      const detail = resolveAgentRunDeepLink(
        (event as CustomEvent<unknown>).detail,
      );
      if (!detail) {
        window.alert("Couldn't open run: workspace, chat, or run ID is missing.");
        return;
      }
      const session = chatSessions.find((item) => item.id === detail.chatId);
      if (!session || session.workspacePath !== detail.workspacePath) {
        window.alert(
          `Couldn't open run ${detail.runId}: its persisted workspace chat is unavailable.`,
        );
        return;
      }
      const generation = persistedRunGeneration.current + 1;
      persistedRunGeneration.current = generation;
      switchSession(detail.chatId);
      setReviewOpen(false);
      setActiveSurface("inspector");
      setPersistedRun({
        ...detail,
        snapshot: null,
        loading: true,
        error: null,
      });
      void hydratePersistedAgentRun(
        detail.workspacePath,
        detail.chatId,
        detail.runId,
      )
        .then((snapshot) => {
          if (persistedRunGeneration.current !== generation) return;
          setPersistedRun({
            ...detail,
            snapshot,
            loading: false,
            error: null,
          });
        })
        .catch((error) => {
          if (persistedRunGeneration.current !== generation) return;
          setPersistedRun({
            ...detail,
            snapshot: null,
            loading: false,
            error: error instanceof Error ? error.message : String(error),
          });
        });
    };
    window.addEventListener("altai:open-agent-run", openAgentRun);
    return () => {
      persistedRunGeneration.current += 1;
      window.removeEventListener("altai:open-agent-run", openAgentRun);
    };
  }, [chatSessions, switchSession]);

  // Session history and open chat tabs are deliberately separate. Selecting a
  // conversation from history opens it in a tab; closing that tab keeps the
  // local conversation available in history instead of deleting it.
  useEffect(() => {
    setOpenChatIds((current) =>
      reconcileOpenChatTabIds({
        openIds: current,
        sessionIds: sessionIds(chatSessions),
        activeSessionId: sessionId,
      }),
    );
  }, [chatSessions, sessionId]);

  const createChatTab = () => {
    const id = newSession();
    setOpenChatIds((current) => openIdsAfterNewChat(current, id));
    setActiveSurface(null);
  };

  const closeChatTab = (chatId: string) => {
    // Last open tab close → dismiss the side chat (not mint another empty tab).
    const closingLastTab =
      openChatIds.length === 1 && openChatIds[0] === chatId;
    if (closingLastTab && onClose) {
      onClose();
      return;
    }
    const result = closeChatTabSelection({
      openIds: openChatIds,
      closingId: chatId,
      activeSessionId: sessionId,
      createSessionId: () => newSession(),
    });
    if (result.focusSessionId) {
      switchSession(result.focusSessionId);
    }
    setOpenChatIds(result.openIds);
    setActiveSurface(null);
  };

  const closeRunInspector = () => {
    persistedRunGeneration.current += 1;
    setPersistedRun(null);
    setActiveSurface(null);
  };

  useEffect(() => {
    const openReview = () => setReviewOpen(true);
    window.addEventListener("altai:open-change-review", openReview);
    return () => window.removeEventListener("altai:open-change-review", openReview);
  }, []);

  return (
    <AiSidePanelFrame
      ref={panelRootRef}
      variant={variant === "workspace" ? "workspace" : "sidebar"}
      topbar={showTopbar ? (
        <WorkspaceTopbar
          variant={variant}
          onOpenStudio={onOpenStudio}
          openChatIds={openChatIds}
          onSelectChat={() => setActiveSurface(null)}
          onCloseChat={closeChatTab}
          onNewChat={createChatTab}
          historyOpen={historyOpen}
          onToggleHistory={() => toggleSurface("history")}
          onOpenSettings={onOpenSettings}
        />
      ) : null}
    >
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
                  aria-label={SIDE_PANEL_CHAT_SESSIONS_ARIA}
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
                aria-label={SIDE_PANEL_RESIZE_HISTORY_LABEL}
                title={SIDE_PANEL_RESIZE_HISTORY_LABEL}
              />
            </>
          ) : null}

          <ResizablePanel
            id="ai-chat-main"
            minSize={variant === "sidebar" ? "0px" : "240px"}
          >
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
                    inspectorOpen={inspectorOpen}
                    inspectorAvailable={inspectorAvailable}
                    onToggleInspector={() => toggleSurface("inspector")}
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
                  {inspectorOpen &&
                  inspectorAvailable &&
                  !showInspectorSidebar ? (
                    <div className="absolute inset-0 z-20 flex bg-card">
                      <RunInspector
                        className="flex w-full"
                        persistedRun={persistedRun}
                        onClose={closeRunInspector}
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
                aria-label={SIDE_PANEL_RESIZE_INSPECTOR_LABEL}
                title={SIDE_PANEL_RESIZE_INSPECTOR_LABEL}
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
                  persistedRun={persistedRun}
                  onClose={closeRunInspector}
                />
              </ResizablePanel>
            </>
          ) : null}
        </ResizablePanelGroup>
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
    </AiSidePanelFrame>
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
  const tabs = chatTabsFromOpenIds(openChatIds, sessions);

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
 * Chat chrome: history + open sessions. Work / Inbox live in primary Desktop
 * navigation; Run details sits with the active chat column.
 */
function WorkspaceTopbar({
  variant,
  onOpenStudio,
  openChatIds,
  onSelectChat,
  onCloseChat,
  onNewChat,
  historyOpen,
  onToggleHistory,
  onOpenSettings,
}: {
  variant: "workspace" | "sidebar";
  onOpenStudio?: () => void;
  openChatIds: string[];
  onSelectChat: () => void;
  onCloseChat: (id: string) => void;
  onNewChat: () => void;
  historyOpen: boolean;
  onToggleHistory: () => void;
  onOpenSettings?: () => void;
}) {
  const historyControl = (
    <IconTooltip label={historyToggleLabel(historyOpen)}>
      <button
        type="button"
        onClick={onToggleHistory}
        aria-label={historyToggleLabel(historyOpen)}
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

  const toggleWindowMaximize = () => {
    if (!hasTauriWindowMetadata()) return;
    void getCurrentWindow().toggleMaximize().catch(() => undefined);
  };

  if (variant === "sidebar") {
    // Sits under the full-width app Header, beside Files/tabs (single h-10 row).
    return (
      <AiPanelTopbar
        aria-label="ALTAI panel chrome"
        className="bg-raised"
        primary={
          <div className="flex h-10 min-w-0 items-center gap-1 px-1.5">
            {historyControl}
            <ChatTabStripBridge
              embedded
              openChatIds={openChatIds}
              onSelect={onSelectChat}
              onCloseChat={onCloseChat}
              onNewChat={onNewChat}
            />
          </div>
        }
      />
    );
  }

  return (
    <AiPanelTopbar
      aria-label="ALTAI panel chrome"
      primary={<div
        className={cn(
          "flex h-10 min-w-0 items-center gap-1.5 px-2.5",
          IS_MAC && "pl-20",
        )}
      >
        <div
          data-tauri-drag-region
          onDoubleClick={toggleWindowMaximize}
          className="h-full min-w-4 flex-1"
          aria-label={SIDE_PANEL_WINDOW_TITLE_BAR_LABEL}
        />
        {historyControl}
        {onOpenSettings ? (
          <IconTooltip label={SIDE_PANEL_SETTINGS_LABEL}>
            <button
              type="button"
              onClick={onOpenSettings}
              aria-label={SIDE_PANEL_SETTINGS_LABEL}
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
      </div>}
    />
  );
}

function RunInspector({
  className,
  onClose,
  persistedRun,
}: {
  className?: string;
  onClose?: () => void;
  persistedRun?: PersistedRunSelection | null;
}) {
  return persistedRun ? (
    <PersistedRunInspector
      className={className}
      onClose={onClose}
      selection={persistedRun}
    />
  ) : (
    <LiveRunInspector className={className} onClose={onClose} />
  );
}

function PersistedRunInspector({
  className,
  onClose,
  selection,
}: {
  className?: string;
  onClose?: () => void;
  selection: PersistedRunSelection;
}) {
  const snapshot = selection.snapshot;
  const outcomeKind = snapshot?.outcome?.kind ?? null;
  const error = selection.error ?? describeTerminalOutcomeAttention(snapshot?.outcome);
  const running = Boolean(snapshot && !snapshot.completed && isAgentRunBusy(snapshot.status));
  const tokenTotal = snapshot
    ? sumRunTokens({ input: snapshot.tokens.input, output: snapshot.tokens.output })
    : 0;
  const statusLabel = selection.loading
    ? "Loading persisted run…"
    : snapshot?.completed
      ? `Persisted ${outcomeKind ?? "terminal"} run`
      : snapshot
        ? `Persisted ${snapshot.status} run`
        : "Persisted run unavailable";

  return (
    <AiRunInspectorFrame
      aria-label={SIDE_PANEL_RUN_DETAILS_ARIA}
      className={className}
      header={
        <RunDetailsHeader
          subtitle={`${statusLabel} · ${selection.runId}`}
          status={error ? "blocked" : running ? "running" : "idle"}
          onClose={onClose}
        />
      }
      summary={
        <RunOverviewCard
          statusPill={
            <span className="rounded bg-muted px-1.5 py-0.5 text-[9px] font-medium text-muted-foreground">
              {selection.loading
                ? "Loading"
                : outcomeKind ?? snapshot?.status ?? "Unavailable"}
            </span>
          }
          tokenLabel={runInspectorUsageTokenLabel(tokenTotal)}
          step={snapshot?.step ?? null}
          metrics={[
            {
              label: INSPECTOR_METRIC_PLAN,
              value: String(snapshot?.verifications.length ?? 0),
            },
            {
              label: INSPECTOR_METRIC_CHANGES,
              value: String(snapshot?.changes.length ?? 0),
            },
            { label: INSPECTOR_METRIC_APPROVALS, value: "0" },
            {
              label: INSPECTOR_METRIC_SUBAGENTS,
              value: String(snapshot?.subagents.length ?? 0),
            },
          ]}
        />
      }
    >
      {selection.loading ? (
        <InspectorEmpty>Loading the exact persisted run…</InspectorEmpty>
      ) : null}
      {error ? <RunBlockedBanner message={error} /> : null}
      {snapshot ? (
        <>
          <InspectorSection title="Identity" summary={selection.runId} count={1} defaultOpen>
            <dl className="space-y-1.5 text-[11px] text-muted-foreground">
              <div><dt className="inline font-medium text-foreground">Workspace: </dt><dd className="inline break-all">{selection.workspacePath}</dd></div>
              <div><dt className="inline font-medium text-foreground">Chat: </dt><dd className="inline break-all">{selection.chatId}</dd></div>
              <div><dt className="inline font-medium text-foreground">Run: </dt><dd className="inline break-all">{selection.runId}</dd></div>
            </dl>
          </InspectorSection>
          {(snapshot.verifications.length > 0 || snapshot.changes.length > 0) ? (
            <InspectorSection
              title="Evidence"
              summary="Verifications and changes"
              count={snapshot.verifications.length + snapshot.changes.length}
              defaultOpen
            >
              <ul className="divide-y divide-border-subtle">
                {snapshot.verifications.map((verification) => (
                  <li key={verification.id} className="py-1.5 text-[11px] text-muted-foreground">
                    {verification.status} · {verification.label}
                  </li>
                ))}
                {snapshot.changes.map((change) => (
                  <li key={`${change.path}:${change.hunkId ?? change.source}`} className="break-all py-1.5 text-[11px] text-muted-foreground">
                    {change.path}
                  </li>
                ))}
              </ul>
            </InspectorSection>
          ) : null}
          {(snapshot.lastResult || snapshot.failures.length > 0) ? (
            <InspectorSection
              title="Result"
              summary="Terminal output"
              count={(snapshot.lastResult ? 1 : 0) + snapshot.failures.length}
              defaultOpen
            >
              {snapshot.lastResult ? (
                <p className="whitespace-pre-wrap text-[11px] text-muted-foreground">
                  {snapshot.lastResult}
                </p>
              ) : null}
              {snapshot.failures.map((failure) => (
                <p key={failure} className="text-[11px] text-destructive">{failure}</p>
              ))}
            </InspectorSection>
          ) : null}
        </>
      ) : null}
    </AiRunInspectorFrame>
  );
}

function LiveRunInspector({ className, onClose }: { className?: string; onClose?: () => void }) {
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

  const completedTodos = countCompletedTodos(todos);
  const filteredActivity = filterActivityByQuery(meta.activity, activityQuery);
  const researchEvents = filterActivityByKind(meta.activity, "research");
  const mcpEvents = filterActivityByKind(meta.activity, "mcp");
  const tokenTotal = sumRunTokens({
    input: meta.tokens.inputTokens,
    output: meta.tokens.outputTokens,
  });
  const running = isAgentRunBusy(meta.status);

  return (
    <AiRunInspectorFrame
      aria-label={SIDE_PANEL_RUN_DETAILS_ARIA}
      className={className}
      header={<RunDetailsHeader
        subtitle={runInspectorHeaderSubtitle(meta.status, meta.step)}
        status={meta.error ? "blocked" : running ? "running" : "idle"}
        onClose={onClose}
        onStop={stopAgent}
      />}
      summary={<RunOverviewCard
          statusPill={<AgentStatusPill announce={false} />}
          tokenLabel={runInspectorUsageTokenLabel(tokenTotal)}
          step={meta.step}
          metrics={[
            {
              label: INSPECTOR_METRIC_PLAN,
              value: planProgressMetricValue(completedTodos, todos.length),
            },
            { label: INSPECTOR_METRIC_CHANGES, value: String(planQueue.length) },
            {
              label: INSPECTOR_METRIC_APPROVALS,
              value: String(meta.pendingApprovals.length),
            },
            {
              label: INSPECTOR_METRIC_SUBAGENTS,
              value: String(meta.activeSubagents.length),
            },
          ]}
        />}
    >

        {meta.error ? <RunBlockedBanner message={meta.error} /> : null}

        {meta.pendingApprovals.length ? (
          <RunActionRequiredSection>
            <ApprovalsInspector
              approvals={meta.pendingApprovals}
              onRespond={respondToApproval}
            />
          </RunActionRequiredSection>
        ) : null}

        {todos.length > 0 ? (
          <InspectorSection
            title="Plan"
            summary={planInspectorSectionSummary(completedTodos, todos.length)}
            count={todos.length}
            defaultOpen={running}
          >
            <TodosInspector done={completedTodos} total={todos.length} todos={todos} />
          </InspectorSection>
        ) : null}

        <InspectorSection
          title={INSPECTOR_ACTIVITY_TITLE}
          summary={INSPECTOR_ACTIVITY_SUMMARY}
          count={meta.activity.length}
          defaultOpen
        >
          <SurfaceSearch
            value={activityQuery}
            onChange={setActivityQuery}
            placeholder={INSPECTOR_ACTIVITY_FILTER_PLACEHOLDER}
            className="mb-1.5"
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

        {planQueue.length > 0 || meta.artifacts.length > 0 ? (
          <InspectorSection
            title={INSPECTOR_CHANGES_TITLE}
            summary={INSPECTOR_CHANGES_SUMMARY}
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
              <div className="my-1.5 border-t border-border-subtle" />
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
          </InspectorSection>
        ) : null}

        {researchEvents.length > 0 || mcpEvents.length > 0 ? (
          <InspectorSection
            title={INSPECTOR_RESEARCH_TITLE}
            summary={INSPECTOR_RESEARCH_SUMMARY}
            count={researchEvents.length + mcpEvents.length}
          >
            {researchEvents.length ? <ResearchInspector events={researchEvents} /> : null}
            {researchEvents.length && mcpEvents.length ? (
              <div className="my-1.5 border-t border-border-subtle" />
            ) : null}
            {mcpEvents.length ? <McpInspector events={mcpEvents} /> : null}
          </InspectorSection>
        ) : null}

        {meta.activeSubagents.length > 0 ? (
          <InspectorSection
            title={INSPECTOR_DELEGATED_TITLE}
            summary={INSPECTOR_DELEGATED_SUMMARY}
            count={meta.activeSubagents.length}
            defaultOpen
          >
            <AgentsInspector tasks={meta.activeSubagents} />
          </InspectorSection>
        ) : null}

        {checkpoints.length > 0 || appliedPlanEdits.length > 0 ? (
          <InspectorSection
            title={INSPECTOR_RECOVERY_TITLE}
            summary={INSPECTOR_RECOVERY_SUMMARY}
            count={checkpoints.length + appliedPlanEdits.length}
          >
            <SnapshotsInspectorBridge
              items={checkpoints}
              applied={appliedPlanEdits}
              setItems={setCheckpoints}
            />
          </InspectorSection>
        ) : null}
    </AiRunInspectorFrame>
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
            setError(result.error ?? PLAN_RESTORE_FALLBACK_ERROR);
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
  inspectorOpen,
  inspectorAvailable,
  onToggleInspector,
  onOpenReview,
  projectTarget,
}: {
  hasComposer: boolean;
  inspectorOpen: boolean;
  inspectorAvailable: boolean;
  onToggleInspector: () => void;
  onOpenReview: () => void;
  projectTarget?: {
    name: string;
    path: string | null;
    kind: "local" | "github" | null;
    onChange: () => void;
  };
}) {
  const sessionId = useChatStore((s) => s.activeSessionId);
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
    <AiChatMainColumn
      planMode={
        <>
          {sessionId || inspectorAvailable ? (
            <div className="flex h-8 shrink-0 items-center gap-1 bg-card px-1.5">
              <div className="min-w-0 flex-1">
                {sessionId ? <TodoSummaryChip sessionId={sessionId} /> : null}
              </div>
              <WorkspaceTopbarActions
                inspectorOpen={inspectorOpen}
                inspectorAvailable={inspectorAvailable}
                onToggleInspector={onToggleInspector}
                renderTooltip={(label, children) => (
                  <IconTooltip label={label}>{children}</IconTooltip>
                )}
              />
            </div>
          ) : null}
          <PlanModeStrip
            active={planModeActive}
            queueLen={reviewQueueLen}
            onReview={() =>
              window.dispatchEvent(new CustomEvent("altai:open-change-review"))
            }
            onExit={() => disablePlanMode()}
          />
        </>
      }
      transcript={
        displayMessages.length === 0 ? (
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
        )
      }
      runChrome={
        <>
          <RunRecoveryActionsBridge />
          <ClarificationChoicesBridge />
          <ChangeReviewBanner
            queueLen={reviewQueueLen}
            onOpen={onOpenReview}
          />
        </>
      }
      composer={
        hasComposer ? (
          <AiInputBar />
        ) : (
          <AiInputBarConnect onAdd={() => void openSettingsWindow("models")} />
        )
      }
      footer={
        projectTarget ? <ChatProjectTarget {...projectTarget} /> : undefined
      }
    />
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
  const canContinue = isRecoverableRunOutcome(outcome);
  const canRetry = isRetryableRunOutcome(outcome);
  if (!warning && !canContinue && !canRetry) return null;

  const { title, detail } = runRecoveryPresentation({
    warningDescription: warning ? describeRunWarning(warning) : null,
    canRetry,
    outcome,
  });

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
        focusInput(runRecoverySteerPrompt(Boolean(warning)));
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
