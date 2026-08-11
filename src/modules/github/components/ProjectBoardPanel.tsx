import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  NewWorkDialog,
  OperationsNavigationShell,
  WorkDetail,
  WorkInbox,
  WorkList,
  type OperationsView,
  type WorkDetailPrimaryAction,
  type WorkInboxRow,
  type WorkListFilterId,
  type WorkListRow,
  WORK_OS_VIEWS,
} from "@altai/agent-ui";
import type { ProjectBoardNavigation } from "@/modules/tabs";

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

/**
 * Work OS tab — list/detail/inbox backed by host `work_*` IPC (work.db).
 */
export function ProjectBoardPanel({ repoRoot, navigation }: Props) {
  const [view, setView] = useState<OperationsView>(
    navigation?.view === "inbox" ? "inbox" : "work",
  );
  const [filter, setFilter] = useState<WorkListFilterId>("my_active");
  const [rows, setRows] = useState<WorkListRow[]>([]);
  const [inboxRows] = useState<WorkInboxRow[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [detail, setDetail] = useState<WorkItemDto | null>(null);
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

  const refresh = useCallback(async () => {
    try {
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
  }, [filter, repoRoot, workspaceName]);

  const loadDetail = useCallback(
    async (workId: string) => {
      setDetailStatus("loading");
      try {
        const item = await invoke<WorkItemDto | null>("work_get", {
          workspacePath: repoRoot,
          workId,
        });
        if (!item) {
          setDetail(null);
          setDetailStatus("not_found");
          return;
        }
        setDetail(item);
        setDetailStatus("ready");
        setLoadError(null);
      } catch (error) {
        setDetail(null);
        setDetailStatus("error");
        setLoadError(error instanceof Error ? error.message : String(error));
      }
    },
    [repoRoot],
  );

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    if (selectedId) void loadDetail(selectedId);
  }, [selectedId, loadDetail]);

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
          next = await invoke<WorkItemDto>("work_start", {
            workspacePath: repoRoot,
            workId: detail.id,
            expectedRevision: detail.revision,
          });
        } else if (action === "open_run") {
          // Until IsanAgent run binding lands, mark attempt ready for review
          // so Accept/Return can be exercised end-to-end.
          next = await invoke<WorkItemDto>("work_ready_for_review", {
            workspacePath: repoRoot,
            workId: detail.id,
            expectedRevision: detail.revision,
          });
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
      } catch (error) {
        setLoadError(error instanceof Error ? error.message : String(error));
      } finally {
        setActionBusy(false);
      }
    },
    [actionBusy, detail, refresh, repoRoot],
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
      {loadError ? (
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
          status="ready"
          rows={inboxRows}
          onOpenWork={(id) => {
            setView("work");
            setSelectedId(id);
          }}
          onGoToWork={() => {
            setSelectedId(null);
            setView("work");
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
