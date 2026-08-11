import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  NewWorkDialog,
  OperationsNavigationShell,
  WorkInbox,
  WorkList,
  type OperationsView,
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
  return state.replaceAll("_", " ");
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

/**
 * Work OS tab — list/inbox backed by host `work_*` IPC (work.db).
 */
export function ProjectBoardPanel({ repoRoot, navigation }: Props) {
  const [view, setView] = useState<OperationsView>(
    navigation?.view === "inbox" ? "inbox" : "work",
  );
  const [filter, setFilter] = useState<WorkListFilterId>("my_active");
  const [rows, setRows] = useState<WorkListRow[]>([]);
  const [inboxRows] = useState<WorkInboxRow[]>([]);
  const [newWorkOpen, setNewWorkOpen] = useState(
    navigation?.action === "new-work",
  );
  const [loadError, setLoadError] = useState<string | null>(null);

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

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    if (!navigation) return;
    if (navigation.action === "new-work") {
      setView("work");
      setNewWorkOpen(true);
      return;
    }
    if (navigation.view === "inbox") setView("inbox");
    else if (navigation.view === "work" || navigation.view === "overview") {
      setView("work");
    } else if (navigation.view === "runs") {
      setView("work");
    }
  }, [navigation]);

  const filteredRows = useMemo(() => rows, [rows]);

  return (
    <OperationsNavigationShell
      view={view}
      onViewChange={setView}
      availableViews={AVAILABLE_VIEWS}
      ariaLabel="Work"
    >
      {loadError ? (
        <p className="text-sm text-amber-600 px-3 py-2" role="status">
          Work store: {loadError}
        </p>
      ) : null}
      {view === "work" ? (
        <WorkList
          status="ready"
          filter={filter}
          onFilterChange={setFilter}
          rows={filteredRows}
          onOpenWork={() => {
            /* Work detail host wiring next. */
          }}
          onNewWork={() => setNewWorkOpen(true)}
          onOpenInbox={() => setView("inbox")}
        />
      ) : null}
      {view === "inbox" ? (
        <WorkInbox
          status="ready"
          rows={inboxRows}
          onOpenWork={() => setView("work")}
          onGoToWork={() => setView("work")}
        />
      ) : null}
      <NewWorkDialog
        open={newWorkOpen}
        projectLabel={workspaceName}
        onClose={() => setNewWorkOpen(false)}
        onCreate={({ title, description, acceptanceCriteria }) => {
          void (async () => {
            try {
              await invoke<WorkItemDto>("work_create", {
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
