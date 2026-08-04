import { Spinner } from "@/components/ui/spinner";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem,
} from "@/components/ui/dropdown-menu";
import { cn } from "@/lib/utils";
import { useAgentRunsStore } from "@/modules/ai/store/agentRunsStore";
import { useChatStore } from "@/modules/ai/store/chatStore";
import { useAgentsStore } from "@/modules/ai/store/agentsStore";
import { usePreferencesStore } from "@/modules/settings/preferences";
import { MODELS, type ModelId } from "@/modules/ai/config";
import { ModelDropdown } from "@/modules/ai/components/ModelDropdown";
import { native, type InstalledSkillInfo } from "@/modules/ai/lib/native";
import type { Assignment, AssignmentStatus } from "@/modules/github/lib/assignments";
import { open } from "@tauri-apps/plugin-dialog";
import {
  ACTIVE_ASSIGNMENT_STATES,
  useAssignmentsStore,
} from "@/modules/github/store/assignmentsStore";
import {
  ArrowDown01Icon,
  ArrowLeft01Icon,
  ArrowReloadHorizontalIcon,
  Attachment02Icon,
  CodeIcon,
  Delete02Icon,
  File01Icon,
  Notebook01Icon,
  PlayIcon,
  TerminalIcon,
  Tick02Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import {
  type FormEvent,
  type ReactNode,
  useEffect,
  useMemo,
  useState,
} from "react";
import {
  AuxiliarySurface,
  ContextSourceToggle,
  SurfaceEmptyState,
  SurfaceSearch,
  SurfaceSectionHeader,
  SurfaceTabs,
  TaskOutcome,
} from "@altai/agent-ui";

const TERMINAL: AssignmentStatus[] = ["done", "failed", "cancelled"];
type TaskFilter = "all" | "active" | "attention" | "finished";

function currentStatus(
  assignment: Assignment,
  run: ReturnType<typeof useAgentRunsStore.getState>["runs"][string] | undefined,
): AssignmentStatus {
  if (TERMINAL.includes(assignment.status) || !run) return assignment.status;
  if (run.completed) {
    if (run.outcome?.kind === "completed") return "done";
    if (run.outcome?.kind === "cancelled") return "cancelled";
    return "failed";
  }
  if (run.status === "thinking" || run.status === "streaming") return "running";
  if (run.status === "awaiting-approval") return "awaiting-approval";
  if (run.status === "error") return "failed";
  return assignment.status;
}

const statusCopy: Record<AssignmentStatus, string> = {
  dispatching: "Starting",
  running: "Working",
  "awaiting-approval": "Needs approval",
  done: "Done",
  failed: "Failed",
  cancelled: "Stopped",
};

const TASK_TEMPLATES = [
  {
    label: "Fix a bug",
    prompt: "Investigate the reported bug, identify the root cause, implement the smallest safe fix, and run the relevant checks.",
  },
  {
    label: "Review changes",
    prompt: "Review the current working-tree changes for correctness, regressions, security risks, and missing tests. Make only clearly necessary fixes and report the findings.",
  },
  {
    label: "Add tests",
    prompt: "Inspect the relevant implementation, add focused tests for the important behavior and edge cases, then run the narrowest useful test command.",
  },
  {
    label: "Refactor safely",
    prompt: "Find the highest-value local refactor in the relevant area. Preserve behavior, keep the diff focused, and verify the result with appropriate checks.",
  },
];

/**
 * A workspace-level task launcher. Each run has a dedicated chat_id, so a
 * long-running job never steals the current conversation or its context.
 */
export function TaskRunsPanel({
  onClose,
  navigation,
}: {
  onClose: () => void;
  navigation?: ReactNode;
}) {
  const assignments = useAssignmentsStore((s) => s.assignments);
  const hydrated = useAssignmentsStore((s) => s.hydrated);
  const dispatching = useAssignmentsStore((s) => s.dispatching);
  const hydrate = useAssignmentsStore((s) => s.hydrate);
  const runTask = useAssignmentsStore((s) => s.runTask);
  const updateStatus = useAssignmentsStore((s) => s.updateStatus);
  const cancel = useAssignmentsStore((s) => s.cancel);
  const remove = useAssignmentsStore((s) => s.remove);
  const runs = useAgentRunsStore((s) => s.runs);
  const activeSessionId = useChatStore((s) => s.activeSessionId);
  const selectedModelId = useChatStore((s) => s.selectedModelId);
  const switchSession = useChatStore((s) => s.switchSession);
  const activeAgentId = useAgentsStore((s) => s.activeId);
  const defaultPermissionMode = usePreferencesStore((s) => s.permissionMode);
  const bypassEnabled = usePreferencesStore((s) => s.bypassPermissionsEnabled);
  const [prompt, setPrompt] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [agentId, setAgentId] = useState(activeAgentId);
  const [permissionMode, setPermissionMode] = useState(defaultPermissionMode);
  const [modelId, setModelId] = useState(selectedModelId);
  const [contextFiles, setContextFiles] = useState<string[]>([]);
  const [includeTerminal, setIncludeTerminal] = useState(false);
  const [includeDiff, setIncludeDiff] = useState(false);
  const [skills, setSkills] = useState<InstalledSkillInfo[]>([]);
  const [selectedSkills, setSelectedSkills] = useState<string[]>([]);
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<TaskFilter>("all");
  const [removeTarget, setRemoveTarget] = useState<Assignment | null>(null);
  const [viewMode, setViewMode] = useState<"queue" | "create">("queue");

  const agents = useMemo(() => {
    const store = useAgentsStore.getState();
    return store.all().filter((agent) => !store.isDisabled(agent.id));
  }, [activeAgentId]);

  const tasks = useMemo(
    () => assignments.filter((assignment) => assignment.source.kind === "task"),
    [assignments],
  );
  const resolvedTasks = useMemo(
    () =>
      tasks
        .map((task) => ({
          task,
          status: currentStatus(task, runs[task.sessionId]),
        }))
        .sort((left, right) => right.task.createdAt - left.task.createdAt),
    [runs, tasks],
  );
  const filterCounts = useMemo(
    () => ({
      all: resolvedTasks.length,
      active: resolvedTasks.filter(({ status }) =>
        ACTIVE_ASSIGNMENT_STATES.includes(status),
      ).length,
      attention: resolvedTasks.filter(
        ({ status }) => status === "awaiting-approval" || status === "failed",
      ).length,
      finished: resolvedTasks.filter(({ status }) =>
        TERMINAL.includes(status),
      ).length,
    }),
    [resolvedTasks],
  );
  const visibleTasks = useMemo(() => {
    const normalizedQuery = query.trim().toLowerCase();
    return resolvedTasks.filter(({ task, status }) => {
      const matchesFilter =
        filter === "all" ||
        (filter === "active" && ACTIVE_ASSIGNMENT_STATES.includes(status)) ||
        (filter === "attention" &&
          (status === "awaiting-approval" || status === "failed")) ||
        (filter === "finished" && TERMINAL.includes(status));
      if (!matchesFilter) return false;
      if (!normalizedQuery) return true;
      const run = runs[task.sessionId];
      return [
        task.title,
        task.source.kind === "task" ? task.source.prompt : "",
        run?.step ?? "",
        run?.lastResult ?? "",
      ]
        .join("\n")
        .toLowerCase()
        .includes(normalizedQuery);
    });
  }, [filter, query, resolvedTasks, runs]);
  const taskGroups = useMemo(
    () => [
      {
        id: "attention",
        title: "Needs attention",
        description: "Runs waiting on you or blocked by an error",
        items: visibleTasks.filter(
          ({ status }) => status === "awaiting-approval" || status === "failed",
        ),
      },
      {
        id: "active",
        title: "In progress",
        description: "Agents currently working in isolated chats",
        items: visibleTasks.filter(
          ({ status }) =>
            status === "dispatching" || status === "running",
        ),
      },
      {
        id: "ready",
        title: "Ready to review",
        description: "Completed runs with transcripts and outcomes",
        items: visibleTasks.filter(({ status }) => status === "done"),
      },
      {
        id: "stopped",
        title: "Stopped",
        description: "Cancelled background work",
        items: visibleTasks.filter(({ status }) => status === "cancelled"),
      },
    ].filter((group) => group.items.length > 0),
    [visibleTasks],
  );

  useEffect(() => {
    void hydrate();
  }, [hydrate]);

  useEffect(() => {
    let mounted = true;
    void native.workspaceCurrentDir()
      .then((workspace) => native.agentListSkills(workspace))
      .then((items) => { if (mounted) setSkills(items); })
      .catch(() => { if (mounted) setSkills([]); });
    return () => { mounted = false; };
  }, []);

  // Persist status changes even when the GitHub board is not mounted. This is
  // especially important for standalone tasks launched from the chat surface.
  useEffect(() => {
    for (const task of tasks) {
      const status = currentStatus(task, runs[task.sessionId]);
      if (status !== task.status) updateStatus(task.id, status);
    }
  }, [runs, tasks, updateStatus]);

  async function start(event: FormEvent) {
    event.preventDefault();
    if (!prompt.trim() || dispatching) return;
    setError(null);
    try {
      const taskPrompt = await addSelectedContext(prompt, {
        files: contextFiles,
        terminal: includeTerminal,
        diff: includeDiff,
      });
      await runTask({
        title: prompt.trim().split("\n")[0],
        prompt: taskPrompt,
        runConfig: { agentId, modelId, permissionMode, skills: selectedSkills },
      });
      setPrompt("");
      setContextFiles([]);
      setIncludeTerminal(false);
      setIncludeDiff(false);
      setViewMode("queue");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Couldn't start the task.");
    }
  }

  const reuseTask = (task: Assignment) => {
    if (task.source.kind !== "task") return;
    setPrompt(task.source.prompt);
    if (task.runConfig?.agentId) setAgentId(task.runConfig.agentId);
    if (task.runConfig?.modelId) {
      const knownModel = MODELS.find(
        (model) => model.id === task.runConfig?.modelId,
      );
      if (knownModel) setModelId(knownModel.id as ModelId);
    }
    if (task.runConfig?.permissionMode) {
      setPermissionMode(task.runConfig.permissionMode);
    }
    setSelectedSkills(task.runConfig?.skills ?? []);
    setViewMode("create");
  };

  const chooseContextFiles = async () => {
    const selected = await open({
      directory: false,
      multiple: true,
      title: "Add files as task context",
    });
    const paths =
      typeof selected === "string"
        ? [selected]
        : Array.isArray(selected)
          ? selected
          : [];
    if (!paths.length) return;
    setContextFiles((current) =>
      Array.from(new Set([...current, ...paths])).slice(0, 12),
    );
  };

  const retryTask = async (task: Assignment) => {
    if (task.source.kind !== "task" || dispatching) return;
    setError(null);
    try {
      await runTask({
        title: task.title.replace(/^🤖\s*/, ""),
        prompt: task.source.prompt,
        runConfig: task.runConfig,
      });
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Couldn't retry the task.");
    }
  };

  const liveContext = useChatStore.getState().live;
  const activeFilePath = liveContext.getActiveFile();
  const activeFileSelected = Boolean(
    activeFilePath && contextFiles.includes(activeFilePath),
  );
  const terminalPrivate = liveContext.isActiveTerminalPrivate();
  const terminalContextAvailable = Boolean(
    !terminalPrivate && liveContext.getTerminalContext()?.trim(),
  );
  const workspaceContextAvailable = Boolean(
    liveContext.getCwd() ?? liveContext.getWorkspaceRoot(),
  );

  return (
    <AuxiliarySurface
      title="Work"
      eyebrow="Workspace work"
      icon={Notebook01Icon}
      subtitle={
        viewMode === "queue"
          ? `${filterCounts.active} working · ${filterCounts.attention} need attention`
          : "Delegate an isolated run without leaving this conversation"
      }
      onClose={onClose}
      navigation={navigation}
      actions={
        viewMode === "queue" ? (
          <button
            type="button"
            onClick={() => setViewMode("create")}
            className="inline-flex h-7 items-center gap-1.5 rounded-md bg-primary px-2.5 text-[9.5px] font-semibold text-primary-foreground hover:bg-primary/85"
          >
            <HugeiconsIcon icon={PlayIcon} size={11} strokeWidth={2} />
            Delegate work
          </button>
        ) : (
          <button
            type="button"
            onClick={() => setViewMode("queue")}
            className="inline-flex h-7 items-center gap-1.5 rounded-md border border-border bg-muted px-2.5 text-[9.5px] font-medium text-foreground hover:bg-accent"
          >
            <HugeiconsIcon icon={ArrowLeft01Icon} size={11} strokeWidth={2} />
            Queue
          </button>
        )
      }
      bodyClassName="overflow-y-auto"
    >
      {viewMode === "queue" ? (
        <div className="shrink-0 space-y-2 border-b border-border-subtle bg-card px-3 py-2.5">
          <SurfaceSearch
            value={query}
            onChange={setQuery}
            placeholder="Search by task, step, or result"
            className="w-full"
          />
          <SurfaceTabs
            label="Filter work runs"
            value={filter}
            onChange={(value) => setFilter(value as TaskFilter)}
            items={[
              { id: "all", label: "All", count: filterCounts.all },
              { id: "active", label: "Live", count: filterCounts.active },
              {
                id: "attention",
                label: "Attention",
                count: filterCounts.attention,
              },
              {
                id: "finished",
                label: "History",
                count: filterCounts.finished,
              },
            ]}
            className="border-0 bg-transparent p-0"
          />
        </div>
      ) : null}
      {viewMode === "create" ? (
      <form onSubmit={start} className="min-h-0 flex-1 overflow-y-auto">
        <section className="border-b border-border-subtle px-3.5 py-3.5">
          <SurfaceSectionHeader
            title="Describe the outcome"
            description="Give the agent a concrete result to deliver and how to verify it."
          />
        <textarea
          id="background-task-prompt"
          value={prompt}
          onChange={(event) => setPrompt(event.target.value)}
          placeholder="Example: Review the auth flow, fix the highest-impact issue, and run the relevant tests."
          className="mt-3 min-h-28 w-full resize-y rounded-lg border border-border bg-muted/55 px-3 py-2.5 text-[11px] leading-relaxed text-foreground outline-none placeholder:text-muted-foreground/65 focus:border-ring focus:ring-2 focus:ring-ring/20"
        />
        <div className="mt-2 grid grid-cols-2 gap-1.5">
          {TASK_TEMPLATES.map((template) => (
            <button
              key={template.label}
              type="button"
              onClick={() => setPrompt(template.prompt)}
              className="min-h-8 rounded-md border border-border bg-card px-2 py-1.5 text-left text-[9.5px] font-medium text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
            >
              {template.label}
            </button>
          ))}
        </div>
        </section>
        <section className="px-3.5 py-3.5">
          <SurfaceSectionHeader
            title="Run configuration"
            description="Choose how the isolated agent should work."
          />
        <div className="mt-3 flex flex-wrap items-center gap-1.5">
          <DropdownMenu>
            <DropdownMenuTrigger className="flex h-6 items-center gap-1 rounded-md border border-border bg-card px-2 text-[10px] text-muted-foreground transition-colors hover:bg-foreground/[0.055]">
              {agents.find((agent) => agent.id === agentId)?.name ?? "Default"}
              <HugeiconsIcon icon={ArrowDown01Icon} size={9} strokeWidth={2} />
            </DropdownMenuTrigger>
            <DropdownMenuContent align="start" className="min-w-28">
              {agents.map((agent) => <DropdownMenuItem key={agent.id} onClick={() => setAgentId(agent.id)} className={cn("text-[11px]", agent.id === agentId && "bg-foreground/[0.085]")}>{agent.name}</DropdownMenuItem>)}
            </DropdownMenuContent>
          </DropdownMenu>
          <ModelDropdown
            value={modelId}
            onChange={setModelId}
            className="h-6 max-w-none border border-border bg-card px-2 text-[10px] hover:bg-accent"
          />
          <DropdownMenu>
            <DropdownMenuTrigger className="flex h-6 items-center gap-1 rounded-md border border-border bg-card px-2 text-[10px] text-muted-foreground transition-colors hover:bg-foreground/[0.055]">
              <span className="truncate">{permissionMode === "ask" ? "Ask" : permissionMode === "auto-edit" ? "Auto-edit" : permissionMode === "plan" ? "Plan" : permissionMode === "bypass" ? "Bypass" : "Ask"}</span>
              <HugeiconsIcon icon={ArrowDown01Icon} size={9} strokeWidth={2} />
            </DropdownMenuTrigger>
            <DropdownMenuContent align="start" className="min-w-32">
              <DropdownMenuItem onClick={() => setPermissionMode("ask")} className={cn("text-[11px]", permissionMode === "ask" && "bg-foreground/[0.085]")}>Ask before changes</DropdownMenuItem>
              <DropdownMenuItem onClick={() => setPermissionMode("auto-edit")} className={cn("text-[11px]", permissionMode === "auto-edit" && "bg-foreground/[0.085]")}>Auto-edit workspace</DropdownMenuItem>
              <DropdownMenuItem onClick={() => setPermissionMode("plan")} className={cn("text-[11px]", permissionMode === "plan" && "bg-foreground/[0.085]")}>Plan mode (read-only)</DropdownMenuItem>
              {bypassEnabled ? <DropdownMenuItem onClick={() => setPermissionMode("bypass")} className={cn("text-[11px]", permissionMode === "bypass" && "bg-foreground/[0.085]")}>Bypass approvals</DropdownMenuItem> : null}
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
        </section>

        <section className="border-t border-border-subtle px-3.5 py-3.5">
          <SurfaceSectionHeader
            title="Context"
            description="Add only the evidence this run needs. Sources are snapshotted when work starts."
            count={contextFiles.length + Number(includeTerminal) + Number(includeDiff)}
          />
          <div className="mt-3 overflow-hidden rounded-lg border border-border bg-card">
            <div className="border-b border-border-subtle p-2.5">
              <div className="flex min-w-0 flex-wrap items-center gap-2">
                <span className="inline-flex size-7 shrink-0 items-center justify-center rounded-md bg-muted text-muted-foreground">
                  <HugeiconsIcon icon={File01Icon} size={13} strokeWidth={1.75} />
                </span>
                <div className="min-w-28 flex-1">
                  <div className="text-[10.5px] font-medium text-foreground">Files</div>
                  <div className="truncate text-[9px] text-muted-foreground">
                    Add the exact files the agent should read first
                  </div>
                </div>
                <div className="ml-auto flex shrink-0 items-center gap-1">
                  <button
                    type="button"
                    onClick={() => {
                      const path = liveContext.getActiveFile();
                      if (path) {
                        setContextFiles((current) =>
                          current.includes(path) ? current : [...current, path],
                        );
                      }
                    }}
                    disabled={!activeFilePath || activeFileSelected}
                    className="h-6 rounded-md px-2 text-[9.5px] font-medium text-muted-foreground hover:bg-accent hover:text-foreground disabled:cursor-not-allowed disabled:opacity-40"
                  >
                    {activeFileSelected ? "Active added" : "Active file"}
                  </button>
                  <button
                    type="button"
                    onClick={() => void chooseContextFiles()}
                    className="inline-flex h-6 items-center gap-1 rounded-md border border-border bg-muted px-2 text-[9.5px] font-medium text-foreground hover:bg-accent"
                  >
                    <HugeiconsIcon icon={Attachment02Icon} size={10} strokeWidth={1.8} />
                    Choose files
                  </button>
                </div>
              </div>
              {contextFiles.length ? (
                <div className="mt-2 flex flex-wrap gap-1">
                  {contextFiles.map((path) => (
                    <span
                      key={path}
                      title={path}
                      className="group inline-flex h-6 max-w-full items-center gap-1 rounded-md border border-border bg-muted/55 pl-2 pr-1 text-[9.5px] text-foreground"
                    >
                      <span className="max-w-44 truncate">{contextFileName(path)}</span>
                      <button
                        type="button"
                        onClick={() =>
                          setContextFiles((current) =>
                            current.filter((item) => item !== path),
                          )
                        }
                        aria-label={`Remove ${contextFileName(path)}`}
                        className="inline-flex size-4 items-center justify-center rounded text-muted-foreground hover:bg-foreground/[0.08] hover:text-foreground"
                      >
                        ×
                      </button>
                    </span>
                  ))}
                </div>
              ) : null}
            </div>
            <ContextSourceToggle
              icon={TerminalIcon}
              label="Terminal output"
              detail={
                terminalPrivate
                  ? "Unavailable while the active terminal is private"
                  : terminalContextAvailable
                    ? "Latest visible output from the active terminal"
                    : "No terminal output available"
              }
              checked={includeTerminal}
              disabled={!terminalContextAvailable}
              onChange={setIncludeTerminal}
            />
            <ContextSourceToggle
              icon={CodeIcon}
              label="Working tree changes"
              detail={
                workspaceContextAvailable
                  ? "Current unstaged Git diff"
                  : "Open a workspace to include Git changes"
              }
              checked={includeDiff}
              disabled={!workspaceContextAvailable}
              onChange={setIncludeDiff}
              className="border-t border-border-subtle"
            />
          </div>
        </section>

        {skills.length ? (
          <section className="border-t border-border-subtle px-3.5 py-3.5">
            <SurfaceSectionHeader
              title="Skills"
              description="Optional playbooks the agent should follow for this run."
              count={selectedSkills.length}
            />
            <div className="mt-3 flex flex-wrap gap-1.5">
              {skills.map((skill) => {
                const selected = selectedSkills.includes(skill.name);
                return (
                  <button
                    key={skill.name}
                    type="button"
                    title={skill.description ?? skill.name}
                    aria-pressed={selected}
                    onClick={() =>
                      setSelectedSkills((current) =>
                        selected
                          ? current.filter((name) => name !== skill.name)
                          : [...current, skill.name],
                      )
                    }
                    className={cn(
                      "inline-flex h-7 items-center gap-1.5 rounded-md border px-2.5 text-[9.5px] font-medium transition-colors",
                      selected
                        ? "border-foreground/15 bg-accent text-foreground"
                        : "border-border bg-card text-muted-foreground hover:bg-accent hover:text-foreground",
                    )}
                  >
                    {selected ? (
                      <HugeiconsIcon icon={Tick02Icon} size={10} strokeWidth={2} />
                    ) : null}
                    {skill.name}
                  </button>
                );
              })}
            </div>
          </section>
        ) : null}

        <section className="border-t border-border-subtle px-3.5 py-3">
        <div className="flex items-center gap-2">
          {error ? <p className="min-w-0 flex-1 text-[10px] text-destructive">{error}</p> : <span className="flex-1" />}
          <button
            type="button"
            onClick={() => setViewMode("queue")}
            className="rounded-md px-2.5 py-1.5 text-[10px] font-medium text-muted-foreground hover:bg-muted hover:text-foreground"
          >
            Cancel
          </button>
          <button
            type="submit"
            disabled={!prompt.trim() || dispatching}
            className="inline-flex items-center gap-1.5 rounded-md bg-primary px-3 py-1.5 text-[10.5px] font-semibold text-primary-foreground transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-45"
          >
            {dispatching ? <Spinner className="size-3" /> : null}
            Run in background
          </button>
        </div>
        </section>
      </form>
      ) : null}

      {viewMode === "queue" ? (
      <div className="min-h-0 flex-1 overflow-y-auto p-3">
        {!hydrated ? (
          <div className="flex items-center justify-center gap-2 py-8 text-[11px] text-muted-foreground"><Spinner className="size-3.5" /> Loading tasks…</div>
        ) : tasks.length === 0 ? (
          <SurfaceEmptyState
            icon={Notebook01Icon}
            title="No background work yet"
            description="Delegate a task to an isolated chat and keep working here while the agent runs."
            action={
              <button
                type="button"
                onClick={() => setViewMode("create")}
                className="rounded-md bg-primary px-3 py-1.5 text-[10px] font-semibold text-primary-foreground"
              >
                Start a background task
              </button>
            }
          />
        ) : visibleTasks.length === 0 ? (
          <div className="border border-dashed border-border px-4 py-8 text-center text-[11px] leading-relaxed text-muted-foreground">
            No tasks match this view.
            <button
              type="button"
              onClick={() => {
                setQuery("");
                setFilter("all");
              }}
              className="ml-1 font-medium text-foreground hover:underline"
            >
              Clear filters
            </button>
          </div>
        ) : (
          <div className="space-y-5">
            {taskGroups.map((group) => (
              <section key={group.id}>
                <SurfaceSectionHeader
                  title={group.title}
                  description={group.description}
                  count={group.items.length}
                  className="mb-2 px-0.5"
                />
                <div className="overflow-hidden rounded-lg border border-border bg-card">
            {group.items.map(({ task, status }, index) => {
              const run = runs[task.sessionId];
              const active = ACTIVE_ASSIGNMENT_STATES.includes(status);
              const tokens = run ? run.tokens.input + run.tokens.output : 0;
              return (
                <article key={task.id} className={cn("p-3", index > 0 && "border-t border-border-subtle")}>
                  <div className="flex items-start gap-2">
                    <span className={cn("mt-1.5 size-1.5 shrink-0 rounded-full", status === "failed" ? "bg-destructive" : status === "done" ? "bg-success" : status === "cancelled" ? "bg-muted-foreground/50" : "animate-pulse bg-info")} />
                    <div className="min-w-0 flex-1">
                      <button
                        type="button"
                        onClick={() => {
                          switchSession(task.sessionId);
                          onClose();
                        }}
                        className="line-clamp-2 text-left text-[11.5px] font-medium leading-snug text-foreground hover:underline"
                      >
                        {task.title.replace(/^🤖\s*/, "")}
                      </button>
                      <p className="mt-1 text-[10px] text-muted-foreground">
                        <span className={cn(status === "failed" && "text-destructive", status === "done" && "text-success")}>{statusCopy[status]}</span>
                        {tokens ? ` · ${tokens >= 1000 ? `${(tokens / 1000).toFixed(1)}k` : tokens} tokens` : ""}
                        {run?.subagents.length ? ` · ${run.subagents.length} agents` : ""}
                        {task.runConfig?.agentId ? ` · ${agents.find((agent) => agent.id === task.runConfig?.agentId)?.name ?? "Custom agent"}` : ""}
                        {task.runConfig?.modelId ? ` · ${MODELS.find((model) => model.id === task.runConfig?.modelId)?.label ?? task.runConfig.modelId}` : ""}
                        {task.runConfig?.skills?.length ? ` · ${task.runConfig.skills.join(", ")}` : ""}
                      </p>
                    </div>
                    <time
                      dateTime={new Date(task.createdAt).toISOString()}
                      className="shrink-0 text-[9px] tabular-nums text-muted-foreground/70"
                    >
                      {formatTaskAge(task.createdAt)}
                    </time>
                  </div>
                  {active && run?.step ? <p className="mt-2 flex items-center gap-1.5 truncate rounded-md bg-muted/70 px-2 py-1.5 text-[10px] text-muted-foreground"><Spinner className="size-3 shrink-0" /> {run.step}</p> : null}
                  {status === "done" && run?.lastResult ? <p className="mt-2 line-clamp-2 text-[10px] leading-relaxed text-muted-foreground">{run.lastResult}</p> : null}
                  {(status === "done" || status === "failed") && run ? (
                    <TaskOutcome
                      changesCount={run.changes.length}
                      checksPassed={run.verifications.filter((v) => v.status === "passed").length}
                      checksFailed={run.verifications.filter((v) => v.status === "failed").length}
                    />
                  ) : null}
                  <div className="mt-2 flex items-center gap-1">
                    <button type="button" onClick={() => { switchSession(task.sessionId); onClose(); }} className="rounded-md px-2 py-1 text-[10px] font-medium text-muted-foreground hover:bg-muted hover:text-foreground">
                      {activeSessionId === task.sessionId ? "Open now" : "Open transcript"}
                    </button>
                    <button
                      type="button"
                      onClick={() => reuseTask(task)}
                      className="rounded-md px-2 py-1 text-[10px] font-medium text-muted-foreground hover:bg-muted hover:text-foreground"
                    >
                      Reuse
                    </button>
                    {status === "failed" ? (
                      <button
                        type="button"
                        disabled={dispatching}
                        onClick={() => void retryTask(task)}
                        className="inline-flex items-center gap-1 rounded-md px-2 py-1 text-[10px] font-medium text-foreground hover:bg-muted disabled:opacity-45"
                      >
                        <HugeiconsIcon icon={ArrowReloadHorizontalIcon} size={10} strokeWidth={2} />
                        Retry
                      </button>
                    ) : null}
                    {active ? (
                      <button type="button" onClick={() => void cancel(task.id)} className="ml-auto rounded-md px-2 py-1 text-[10px] font-medium text-destructive hover:bg-destructive/10">
                        Stop
                      </button>
                    ) : (
                      <button
                        type="button"
                        onClick={() => setRemoveTarget(task)}
                        aria-label={`Remove ${task.title.replace(/^🤖\s*/, "")}`}
                        className="ml-auto inline-flex size-6 items-center justify-center rounded-md text-muted-foreground/70 hover:bg-destructive/10 hover:text-destructive"
                      >
                        <HugeiconsIcon icon={Delete02Icon} size={11} strokeWidth={1.8} />
                      </button>
                    )}
                  </div>
                </article>
              );
            })}
                </div>
              </section>
            ))}
          </div>
        )}
      </div>
      ) : null}
      <AlertDialog
        open={removeTarget !== null}
        onOpenChange={(open) => {
          if (!open) setRemoveTarget(null);
        }}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Remove background task?</AlertDialogTitle>
            <AlertDialogDescription>
              This removes the task card, its run state, and the dedicated transcript.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <div className="line-clamp-2 rounded-md bg-muted px-3 py-2 text-[11px] text-muted-foreground">
            {removeTarget?.title.replace(/^🤖\s*/, "")}
          </div>
          <AlertDialogFooter>
            <AlertDialogCancel>Keep task</AlertDialogCancel>
            <AlertDialogAction
              variant="destructive"
              onClick={() => {
                if (removeTarget) void remove(removeTarget.id);
                setRemoveTarget(null);
              }}
            >
              Remove task
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </AuxiliarySurface>
  );
}

function formatTaskAge(timestamp: number): string {
  const minutes = Math.max(0, Math.floor((Date.now() - timestamp) / 60_000));
  if (minutes < 1) return "now";
  if (minutes < 60) return `${minutes}m`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h`;
  return `${Math.floor(hours / 24)}d`;
}

function contextFileName(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).pop() ?? path;
}

async function addSelectedContext(
  prompt: string,
  selected: { files: string[]; terminal: boolean; diff: boolean },
): Promise<string> {
  const live = useChatStore.getState().live;
  const blocks: string[] = [];
  for (const path of selected.files) {
    try {
      const result = await native.readFile(path, {
        enforceIsanagentignore: true,
      });
      if (result.kind === "text") {
        blocks.push(
          `<context-file path="${path}">\n${result.content.slice(0, 60_000)}\n</context-file>`,
        );
      }
    } catch {
      /* unavailable files simply stay out of the task */
    }
  }
  if (selected.terminal && !live.isActiveTerminalPrivate()) {
    const output = live.getTerminalContext();
    if (output?.trim()) blocks.push(`<terminal-context>\n${output.trim().slice(0, 60_000)}\n</terminal-context>`);
  }
  if (selected.diff) {
    const cwd = live.getCwd() ?? live.getWorkspaceRoot();
    if (cwd) {
      try {
        const repo = await native.gitResolveRepo(cwd);
        if (repo) {
          const diff = await native.gitDiff(repo.repoRoot, null, false);
          if (diff.diffText.trim()) blocks.push(`<working-tree-diff${diff.truncated ? ' truncated="true"' : ""}>\n${diff.diffText.slice(0, 80_000)}\n</working-tree-diff>`);
        }
      } catch { /* non-git workspaces have no diff to include */ }
    }
  }
  return blocks.length ? `${prompt.trim()}\n\n<selected-context>\n${blocks.join("\n\n")}\n</selected-context>` : prompt;
}


