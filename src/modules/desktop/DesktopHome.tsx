import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { formatRelativeTime, WorkInbox, type WorkInboxRow } from "@altai/agent-ui";
import type {
  WorkAttempt,
  WorkInboxItem,
  WorkItem,
  WorkRun,
} from "@altai/host-contract";
import { EmptyState } from "@/components/altai";
import { useChatStore } from "@/modules/ai/store/chatStore";
import { native } from "@/modules/ai/lib/native";
import { WORK_INBOX_INVALIDATION_EVENTS } from "@/modules/ai/lib/workInboxAttention";
import { OperationsStatusBar } from "@/modules/operations";
import {
  RunsHubSection,
  WorkBoard,
  WorkDetailPanel,
  projectRunsHub,
  projectWorkBoard,
  type WorkBoardRow,
  type WorkRunRow,
} from "@/modules/work-board";

type LoadStatus = "loading" | "ready" | "error";

type Props = {
  workspacePath: string | null;
  workspaceName: string;
  onOpenWork: (workId: string) => void;
  onOpenInbox: () => void;
  onNewWork: () => void;
  onInboxCountChange?: (count: number) => void;
};

export function toHomeInboxRow(item: WorkInboxItem): WorkInboxRow {
  return {
    id: item.id,
    workId: item.workId,
    kind: item.kind,
    title: item.title,
    why: item.why,
    ageLabel: formatRelativeTime(item.createdAtMs),
  };
}

/** M5-A Home projection: canonical Inbox attention beside My active Work. */
export function DesktopHome({
  workspacePath,
  workspaceName,
  onOpenWork,
  onOpenInbox,
  onNewWork,
  onInboxCountChange,
}: Props) {
  const [status, setStatus] = useState<LoadStatus>("loading");
  const [error, setError] = useState<string | null>(null);
  const [work, setWork] = useState<WorkItem[]>([]);
  const [attempts, setAttempts] = useState<WorkAttempt[]>([]);
  const [runs, setRuns] = useState<WorkRun[]>([]);
  const [inbox, setInbox] = useState<WorkInboxItem[]>([]);
  const [selectedWorkId, setSelectedWorkId] = useState<string | null>(null);
  const switchSession = useChatStore((state) => state.switchSession);
  const requestGeneration = useRef(0);
  const hasLoaded = useRef(false);
  const onInboxCountChangeRef = useRef(onInboxCountChange);

  useEffect(() => {
    onInboxCountChangeRef.current = onInboxCountChange;
  }, [onInboxCountChange]);

  const refresh = useCallback(async () => {
    if (!workspacePath) return;
    const generation = ++requestGeneration.current;
    if (!hasLoaded.current) setStatus("loading");
    try {
      const [nextWork, nextInbox] = await Promise.all([
        invoke<WorkItem[]>("work_list", {
          workspacePath,
          filter: "my_active",
        }),
        invoke<WorkInboxItem[]>("work_inbox_list", { workspacePath }),
      ]);
      // The board's second axis: the latest attempt's phase per Work.
      const nextAttempts = (
        await Promise.all(
          nextWork.map((item) =>
            native.workAttempts(item.id, workspacePath).catch(() => [] as WorkAttempt[]),
          ),
        )
      ).flat();
      // The Runs hub's page: one projection, no per-Work fan-out. A failure
      // degrades to the previous page, not a failed Home.
      const nextRuns = await native
        .workRuns(20, workspacePath)
        .catch(() => [] as WorkRun[]);
      if (generation !== requestGeneration.current) return;
      setWork(nextWork);
      setAttempts(nextAttempts);
      setRuns(nextRuns);
      setInbox(nextInbox);
      setError(null);
      setStatus("ready");
      hasLoaded.current = true;
      onInboxCountChangeRef.current?.(nextInbox.length);
    } catch (loadError) {
      if (generation !== requestGeneration.current) return;
      setError(
        loadError instanceof Error ? loadError.message : String(loadError),
      );
      setStatus("error");
      hasLoaded.current = true;
    }
  }, [workspacePath]);

  useEffect(() => {
    requestGeneration.current += 1;
    hasLoaded.current = false;
    setSelectedWorkId(null);
    setWork([]);
    setAttempts([]);
    setRuns([]);
    setInbox([]);
    setError(null);
    if (!workspacePath) {
      setStatus("ready");
      onInboxCountChangeRef.current?.(0);
      return;
    }
    void refresh();
    const onInvalidated = () => void refresh();
    WORK_INBOX_INVALIDATION_EVENTS.forEach((eventName) =>
      window.addEventListener(eventName, onInvalidated),
    );
    const poll = window.setInterval(onInvalidated, 5_000);
    return () => {
      requestGeneration.current += 1;
      window.clearInterval(poll);
      WORK_INBOX_INVALIDATION_EVENTS.forEach((eventName) =>
        window.removeEventListener(eventName, onInvalidated),
      );
    };
  }, [refresh, workspacePath]);

  const boardRows = useMemo<WorkBoardRow[]>(
    () =>
      projectWorkBoard({
        work,
        attempts,
        inbox,
        formatUpdated: formatRelativeTime,
      }),
    [work, attempts, inbox],
  );
  const inboxRows = useMemo(() => inbox.map(toHomeInboxRow), [inbox]);
  const runRows = useMemo<WorkRunRow[]>(() => projectRunsHub(runs), [runs]);

  if (!workspacePath) {
    return (
      <section
        aria-labelledby="desktop-home-heading"
        className="flex h-full min-h-0 flex-col bg-background"
      >
        <header className="border-b border-border-subtle px-5 py-4">
          <h2
            id="desktop-home-heading"
            tabIndex={-1}
            className="text-[15px] font-semibold text-foreground"
          >
            Home
          </h2>
          <p className="mt-0.5 text-[11px] text-muted-foreground">
            What needs you and the Work already in motion.
          </p>
          <OperationsStatusBar />
        </header>
        <EmptyState
          className="min-h-0 flex-1"
          glow={false}
          title="Choose a project"
          description="Home uses the current project’s Work and Inbox."
        />
      </section>
    );
  }

  return (
    <section
      aria-labelledby="desktop-home-heading"
      className="flex h-full min-h-0 flex-col bg-background"
    >
      <header className="shrink-0 border-b border-border-subtle px-5 py-3.5">
        <p className="text-[10px] font-medium uppercase tracking-[0.14em] text-muted-foreground">
          {workspaceName}
        </p>
        <h2
          id="desktop-home-heading"
          tabIndex={-1}
          className="mt-0.5 text-[15px] font-semibold text-foreground"
        >
          Home
        </h2>
        <OperationsStatusBar />
      </header>
      <div className="grid min-h-0 flex-1 grid-cols-1 gap-px overflow-hidden bg-border-subtle lg:grid-cols-[minmax(240px,0.3fr)_minmax(420px,0.7fr)]">
        <div className="flex min-h-[240px] flex-col overflow-hidden bg-card">
          <WorkInbox
            status={status}
            rows={inboxRows}
            errorMessage={error ?? undefined}
            onRetry={() => void refresh()}
            onOpenWork={onOpenWork}
            onGoToWork={onOpenInbox}
            className="min-h-0 flex-1"
          />
          <RunsHubSection
            rows={runRows}
            onOpenWork={setSelectedWorkId}
          />
        </div>
        {selectedWorkId ? (
          <WorkDetailPanel
            workspacePath={workspacePath}
            workspaceName={workspaceName}
            workId={selectedWorkId}
            onBack={() => setSelectedWorkId(null)}
            onOpenWork={setSelectedWorkId}
            onOpenTranscript={switchSession}
            className="min-h-[280px]"
          />
        ) : (
          <WorkBoard
            status={status}
            rows={boardRows}
            onOpenWork={setSelectedWorkId}
            onNewWork={onNewWork}
            onOpenInbox={onOpenInbox}
            errorMessage={error ?? undefined}
            onRetry={() => void refresh()}
            title="My active"
            className="min-h-[280px]"
          />
        )}
      </div>
    </section>
  );
}
