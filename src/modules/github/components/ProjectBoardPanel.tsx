import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  NewWorkDialog,
  OperationsNavigationShell,
  formatRelativeTime,
  WorkDetail,
  WorkInbox,
  WorkList,
  type OperationsView,
  type WorkDetailAttemptRow,
  type WorkDetailPrimaryAction,
  type WorkInboxRow,
  type WorkListFilterId,
  type WorkListRow,
  WORK_OS_VIEWS,
} from "@altai/agent-ui";
import type { WorkInboxItem } from "@altai/host-contract";
import { WorkInboxRequestGate } from "@/modules/ai/lib/workInboxAttention";
import type { ProjectBoardNavigation } from "@/modules/tabs";
import {
  dispatchToSessionWithRunRef,
  requestStop,
  useChatStore,
} from "@/modules/ai/store/chatStore";

type Props = {
  repoRoot: string;
  navigation?: ProjectBoardNavigation;
};

/** Milestone 1: Work + Inbox only (SCREENS.md). Legacy Overview/Runs removed. */
const AVAILABLE_VIEWS: readonly OperationsView[] = WORK_OS_VIEWS;

type WorkItemDto = {
  id: string;
  projectId: string;
  title: string;
  description: string;
  acceptanceCriteria: string;
  state: string;
  assigneeRef?: string | null;
  blocker?: string | null;
  revision: number;
  createdAtMs: number;
  updatedAtMs: number;
};

type WorkAttemptDto = {
  id: string;
  workId: string;
  number: number;
  role: string;
  phase: string;
  chatId?: string | null;
  sessionId?: string | null;
  runId?: string | null;
  inputJson?: string | null;
  resultJson?: string | null;
  createdAtMs: number;
  updatedAtMs: number;
};

type WorkStartResultDto = {
  work: WorkItemDto;
  attempt: WorkAttemptDto;
};

type WorkReconcileResultDto = {
  changedWorkIds: string[];
};

function stateLabel(state: string): string {
  return state.split("_").join(" ");
}

function toListRow(item: WorkItemDto, projectLabel: string): WorkListRow {
  return {
    id: item.id,
    title: item.title,
    projectLabel,
    stateLabel: stateLabel(item.state),
    attemptLabel: "—",
    updatedLabel: "recent",
  };
}

function primaryActionsFor(state: string): WorkDetailPrimaryAction[] {
  switch (state) {
    case "backlog":
      return ["ready", "start"];
    case "ready":
      return ["start"];
    case "in_progress":
      return ["open_run"];
    case "in_review":
      return ["accept", "return"];
    case "done":
    case "cancelled":
      return ["reopen"];
    default:
      return [];
  }
}

function attemptPrompt(item: WorkItemDto): string {
  const sections = [`Deliver this Work outcome: ${item.title}`];
  if (item.description.trim()) {
    sections.push(`Description:\n${item.description.trim()}`);
  }
  if (item.acceptanceCriteria.trim()) {
    sections.push(
      `Acceptance criteria:\n${item.acceptanceCriteria.trim()}`,
    );
  }
  sections.push(
    "Work in the current workspace, verify the result, and report the evidence. Do not mark the Work accepted; human review owns that decision.",
  );
  return sections.join("\n\n");
}

function openAttemptRun(attempt: WorkAttemptDto, workspacePath: string): void {
  if (!attempt.chatId || !attempt.runId) return;
  window.dispatchEvent(
    new CustomEvent("altai:open-agent-run", {
      detail: { workspacePath, chatId: attempt.chatId, runId: attempt.runId },
    }),
  );
}

/**
 * Work OS tab — list/detail/inbox backed by host `work_*` IPC (work.db).
 */
export function ProjectBoardPanel({ repoRoot, navigation }: Props) {
  const [view, setView] = useState<OperationsView>(
    navigation?.view === "inbox" ? "inbox" : "work",
  );
  const [filter, setFilter] = useState<WorkListFilterId>("my_active");
  const [rows, setRows] = useState<WorkListRow[]>([]);
  const [inboxRows, setInboxRows] = useState<WorkInboxRow[]>([]);
  const [inboxStatus, setInboxStatus] = useState<
    "loading" | "ready" | "error"
  >("loading");
  const [inboxError, setInboxError] = useState<string | null>(null);
  const inboxRequestGate = useRef(new WorkInboxRequestGate(repoRoot));
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [detail, setDetail] = useState<WorkItemDto | null>(null);
  const [attempts, setAttempts] = useState<WorkAttemptDto[]>([]);
  const [detailStatus, setDetailStatus] = useState<
    "loading" | "ready" | "error" | "not_found"
  >("ready");
  const [newWorkOpen, setNewWorkOpen] = useState(
    navigation?.action === "new-work",
  );
  const [loadError, setLoadError] = useState<string | null>(null);
  const [actionBusy, setActionBusy] = useState(false);

  const workspaceName =
    repoRoot.split(/[\\/]/).filter(Boolean).pop() ?? "Local workspace";

  const reconcileAttempts = useCallback(
    () =>
      invoke<WorkReconcileResultDto>("work_attempt_reconcile", {
        workspacePath: repoRoot,
      }),
    [repoRoot],
  );

  const refresh = useCallback(async () => {
    try {
      await reconcileAttempts();
      const items = await invoke<WorkItemDto[]>("work_list", {
        workspacePath: repoRoot,
        filter:
          filter === "my_active"
            ? "my_active"
            : filter === "review"
              ? "review"
              : filter === "backlog"
                ? "backlog"
                : "done",
      });
      setRows(items.map((item) => toListRow(item, workspaceName)));
      setLoadError(null);
    } catch (error) {
      setLoadError(error instanceof Error ? error.message : String(error));
    }
  }, [filter, reconcileAttempts, repoRoot, workspaceName]);

  const refreshInbox = useCallback(async () => {
    const request = inboxRequestGate.current.begin(repoRoot);
    if (!inboxRequestGate.current.isCurrent(request)) return;
    setInboxStatus("loading");
    try {
      await reconcileAttempts();
      const items = await invoke<WorkInboxItem[]>("work_inbox_list", {
        workspacePath: repoRoot,
      });
      if (!inboxRequestGate.current.isCurrent(request)) return;
      setInboxRows(
        items.map((item) => ({
          id: item.id,
          workId: item.workId,
          kind: item.kind,
          title: item.title,
          why: item.why,
          ageLabel: formatRelativeTime(item.createdAtMs),
        })),
      );
      setInboxError(null);
      setInboxStatus("ready");
      window.dispatchEvent(new CustomEvent("altai:work-inbox-changed"));
    } catch (error) {
      if (!inboxRequestGate.current.isCurrent(request)) return;
      setInboxError(error instanceof Error ? error.message : String(error));
      setInboxStatus("error");
    }
  }, [reconcileAttempts, repoRoot]);

  useLayoutEffect(() => {
    inboxRequestGate.current.reset(repoRoot);
    setInboxRows([]);
    setInboxError(null);
    setInboxStatus("loading");
    return () => {
      if (inboxRequestGate.current.ownsWorkspace(repoRoot)) {
        inboxRequestGate.current.reset(repoRoot);
      }
    };
  }, [repoRoot]);

  const loadDetail = useCallback(
    async (workId: string) => {
      setDetailStatus("loading");
      try {
        await reconcileAttempts();
        const [item, attemptRows] = await Promise.all([
          invoke<WorkItemDto | null>("work_get", {
            workspacePath: repoRoot,
            workId,
          }),
          invoke<WorkAttemptDto[]>("work_attempts", {
            workspacePath: repoRoot,
            workId,
          }),
        ]);
        if (!item) {
          setDetail(null);
          setAttempts([]);
          setDetailStatus("not_found");
          return;
        }
        setDetail(item);
        setAttempts(attemptRows);
        setDetailStatus("ready");
        setLoadError(null);
      } catch (error) {
        setDetail(null);
        setAttempts([]);
        setDetailStatus("error");
        setLoadError(error instanceof Error ? error.message : String(error));
      }
    },
    [reconcileAttempts, repoRoot],
  );

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    if (view === "inbox") void refreshInbox();
  }, [refreshInbox, view]);

  useEffect(() => {
    if (selectedId) void loadDetail(selectedId);
  }, [selectedId, loadDetail]);

  useEffect(() => {
    const journaledTerminal = () => {
      void refresh();
      void refreshInbox();
      if (selectedId) void loadDetail(selectedId);
    };
    window.addEventListener(
      "altai:agent-terminal-journaled",
      journaledTerminal,
    );
    return () =>
      window.removeEventListener(
        "altai:agent-terminal-journaled",
        journaledTerminal,
      );
  }, [loadDetail, refresh, refreshInbox, selectedId]);

  useEffect(() => {
    const reconcileTimer = window.setInterval(() => {
      void reconcileAttempts()
        .then((result) => {
          if (!result.changedWorkIds.length) return;
          void refresh();
          void refreshInbox();
          if (selectedId) void loadDetail(selectedId);
        })
        .catch((error) => {
          setLoadError(error instanceof Error ? error.message : String(error));
        });
    }, 5_000);
    return () => window.clearInterval(reconcileTimer);
  }, [loadDetail, reconcileAttempts, refresh, refreshInbox, selectedId]);

  useEffect(() => {
    if (!navigation) return;
    if (navigation.action === "new-work") {
      setView("work");
      setSelectedId(null);
      setNewWorkOpen(true);
      return;
    }
    if (navigation.view === "inbox") {
      setView("inbox");
      setSelectedId(null);
    } else if (navigation.view === "work" || navigation.view === "overview") {
      setView("work");
    } else if (navigation.view === "runs") {
      setView("work");
    }
  }, [navigation]);

  const filteredRows = useMemo(() => rows, [rows]);

  const runAction = useCallback(
    async (action: WorkDetailPrimaryAction) => {
      if (!detail || actionBusy) return;
      setActionBusy(true);
      try {
        let next: WorkItemDto | null = null;
        if (action === "ready") {
          next = await invoke<WorkItemDto>("work_transition", {
            workspacePath: repoRoot,
            workId: detail.id,
            expectedRevision: detail.revision,
            nextState: "ready",
          });
        } else if (action === "start") {
          const chat = useChatStore.getState();
          const chatId = chat.createBackgroundSession(detail.title);
          chat.setSessionWorkspace(chatId, {
            path: repoRoot,
            kind: "local",
          });
          let started: WorkStartResultDto;
          try {
            started = await invoke<WorkStartResultDto>(
              "work_start_attempt",
              {
                workspacePath: repoRoot,
                workId: detail.id,
                expectedRevision: detail.revision,
                chatId,
                sessionId: chatId,
              },
            );
          } catch (error) {
            chat.deleteSession(chatId);
            throw error;
          }
          next = started.work;
          setDetail(started.work);
          setAttempts((current) => [
            started.attempt,
            ...current.filter((attempt) => attempt.id !== started.attempt.id),
          ]);
          const run = await dispatchToSessionWithRunRef(
            attemptPrompt(detail),
            chatId,
            { workspacePath: repoRoot },
          );
          if (!run) {
            chat.deleteSession(chatId);
            next = await invoke<WorkItemDto | null>("work_attempt_finish", {
              workspacePath: repoRoot,
              attemptId: started.attempt.id,
              runId: null,
              phase: "failed",
              resultJson: JSON.stringify({
                kind: "failed",
                failure: "runtime rejected the Work attempt",
              }),
            });
            if (next) setDetail(next);
            void refresh();
            void loadDetail(detail.id);
            throw new Error(
              "Couldn't start the agent run — check the selected model and API key.",
            );
          }

          let bound: WorkAttemptDto;
          try {
            bound = await invoke<WorkAttemptDto>("work_attempt_bind", {
              workspacePath: repoRoot,
              attemptId: started.attempt.id,
              chatId: run.chatId,
              sessionId: run.chatId,
              runId: run.runId,
            });
          } catch (error) {
            await requestStop(chatId).catch(() => false);
            await invoke<WorkItemDto | null>("work_attempt_finish", {
              workspacePath: repoRoot,
              attemptId: started.attempt.id,
              runId: null,
              phase: "failed",
              resultJson: JSON.stringify({
                kind: "failed",
                failure: "could not bind the accepted agent run",
              }),
            }).catch(() => null);
            void refresh();
            void loadDetail(detail.id);
            throw error;
          }
          setAttempts((current) => [
            bound,
            ...current.filter((attempt) => attempt.id !== bound.id),
          ]);

          // Journal delivery may beat either IPC acknowledgement. Ask the
          // host to reconcile the explicit workspace after the idempotent
          // bind; renderer memory is not the completion source of truth.
          const reconciled = await reconcileAttempts();
          if (reconciled.changedWorkIds.includes(detail.id)) {
            const [updated, attemptRows] = await Promise.all([
              invoke<WorkItemDto | null>("work_get", {
                workspacePath: repoRoot,
                workId: detail.id,
              }),
              invoke<WorkAttemptDto[]>("work_attempts", {
                workspacePath: repoRoot,
                workId: detail.id,
              }),
            ]);
            setAttempts(attemptRows);
            if (updated) next = updated;
          }
        } else if (action === "open_run") {
          const currentAttempt = attempts.find(
            (attempt) => attempt.chatId && attempt.runId,
          );
          if (!currentAttempt) {
            throw new Error("This Attempt has no bound run yet.");
          }
          openAttemptRun(currentAttempt, repoRoot);
        } else if (action === "accept") {
          next = await invoke<WorkItemDto>("work_review", {
            workspacePath: repoRoot,
            workId: detail.id,
            expectedRevision: detail.revision,
            accept: true,
            guidance: "",
          });
        } else if (action === "return") {
          const guidance =
            window.prompt("Return guidance (required for the next attempt):") ??
            "";
          if (!guidance.trim()) {
            setActionBusy(false);
            return;
          }
          next = await invoke<WorkItemDto>("work_review", {
            workspacePath: repoRoot,
            workId: detail.id,
            expectedRevision: detail.revision,
            accept: false,
            guidance: guidance.trim(),
          });
        } else if (action === "reopen") {
          next = await invoke<WorkItemDto>("work_transition", {
            workspacePath: repoRoot,
            workId: detail.id,
            expectedRevision: detail.revision,
            nextState: "ready",
          });
        }
        if (next) {
          setDetail(next);
          setDetailStatus("ready");
        }
        await refresh();
        await refreshInbox();
      } catch (error) {
        setLoadError(error instanceof Error ? error.message : String(error));
      } finally {
        setActionBusy(false);
      }
    },
    [
      actionBusy,
      attempts,
      detail,
      loadDetail,
      reconcileAttempts,
      refresh,
      refreshInbox,
      repoRoot,
    ],
  );

  const showDetail = view === "work" && selectedId !== null;

  return (
    <OperationsNavigationShell
      view={view}
      onViewChange={(next) => {
        setView(next);
        setSelectedId(null);
      }}
      availableViews={AVAILABLE_VIEWS}
      ariaLabel="Work"
    >
      {view === "work" && loadError ? (
        <p className="text-sm text-amber-600 px-3 py-2" role="status">
          Work store: {loadError}
        </p>
      ) : null}
      {view === "work" && !showDetail ? (
        <WorkList
          status="ready"
          filter={filter}
          onFilterChange={setFilter}
          rows={filteredRows}
          onOpenWork={(id) => setSelectedId(id)}
          onNewWork={() => setNewWorkOpen(true)}
          onOpenInbox={() => {
            setSelectedId(null);
            setView("inbox");
          }}
        />
      ) : null}
      {showDetail ? (
        <WorkDetail
          status={detailStatus}
          title={detail?.title}
          stateLabel={detail ? stateLabel(detail.state) : undefined}
          projectLabel={workspaceName}
          description={detail?.description}
          acceptanceCriteria={detail?.acceptanceCriteria}
          blocker={detail?.blocker}
          attempts={attempts.map<WorkDetailAttemptRow>((attempt) => ({
            id: attempt.id,
            label: `#${attempt.number} ${attempt.role}`,
            phaseLabel: stateLabel(attempt.phase),
            ...(attempt.chatId && attempt.runId
              ? { onOpenRun: () => openAttemptRun(attempt, repoRoot) }
              : {}),
          }))}
          primaryActions={
            detail && !actionBusy ? primaryActionsFor(detail.state) : []
          }
          onPrimaryAction={(action) => {
            void runAction(action);
          }}
          onBack={() => setSelectedId(null)}
          onCopyId={
            detail
              ? () => {
                  void navigator.clipboard.writeText(detail.id);
                }
              : undefined
          }
          onRetry={
            selectedId
              ? () => {
                  void loadDetail(selectedId);
                }
              : undefined
          }
          errorMessage={loadError ?? undefined}
        />
      ) : null}
      {view === "inbox" ? (
        <WorkInbox
          status={inboxStatus}
          rows={inboxRows}
          onOpenWork={(id) => {
            setView("work");
            setSelectedId(id);
          }}
          onGoToWork={() => {
            setSelectedId(null);
            setView("work");
          }}
          errorMessage={inboxError ?? undefined}
          onRetry={() => {
            void refreshInbox();
          }}
        />
      ) : null}
      <NewWorkDialog
        open={newWorkOpen}
        projectLabel={workspaceName}
        onClose={() => setNewWorkOpen(false)}
        onCreate={({ title, description, acceptanceCriteria }) => {
          void (async () => {
            try {
              const created = await invoke<WorkItemDto>("work_create", {
                args: {
                  workspacePath: repoRoot,
                  title,
                  description,
                  acceptanceCriteria,
                },
              });
              setFilter("backlog");
              setNewWorkOpen(false);
              await refresh();
              setSelectedId(created.id);
            } catch (error) {
              setLoadError(
                error instanceof Error ? error.message : String(error),
              );
            }
          })();
        }}
      />
    </OperationsNavigationShell>
  );
}
