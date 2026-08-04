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
  Notebook01Icon,
  PlayIcon,
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
  SurfaceEmptyState,
  SurfaceSectionHeader,
  CreateFormActions,
  PromptEditorSection,
  SurfaceFilteredEmpty,
  SurfaceFilterToolbar,
  SurfacePrimaryAction,
  SurfaceSecondaryAction,
  TaskContextSources,
  TaskRunCard,
  TaskRunConfigSection,
  TaskSkillChips,
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
          <SurfacePrimaryAction onClick={() => setViewMode("create")}>
            <HugeiconsIcon icon={PlayIcon} size={11} strokeWidth={2} />
            Delegate work
          </SurfacePrimaryAction>
        ) : (
          <SurfaceSecondaryAction onClick={() => setViewMode("queue")}>
            <HugeiconsIcon icon={ArrowLeft01Icon} size={11} strokeWidth={2} />
            Queue
          </SurfaceSecondaryAction>
        )
      }
      bodyClassName="overflow-y-auto"
    >
      {viewMode === "queue" ? (
        <SurfaceFilterToolbar
          query={query}
          onQueryChange={setQuery}
          searchPlaceholder="Search by task, step, or result"
          tabsLabel="Filter work runs"
          tabValue={filter}
          onTabChange={(value) => setFilter(value as TaskFilter)}
          tabs={[
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
        />
      ) : null}
      {viewMode === "create" ? (
      <form onSubmit={start} className="min-h-0 flex-1 overflow-y-auto">
        <PromptEditorSection
          title="Describe the outcome"
          description="Give the agent a concrete result to deliver and how to verify it."
          value={prompt}
          onChange={setPrompt}
          textareaId="background-task-prompt"
          placeholder="Example: Review the auth flow, fix the highest-impact issue, and run the relevant tests."
          templates={TASK_TEMPLATES.map((template) => ({
            label: template.label,
            value: template.prompt,
          }))}
        />
        <TaskRunConfigSection>
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
        </TaskRunConfigSection>

        <TaskContextSources
          files={contextFiles}
          onAddActiveFile={() => {
            const path = liveContext.getActiveFile();
            if (path) {
              setContextFiles((current) =>
                current.includes(path) ? current : [...current, path],
              );
            }
          }}
          onChooseFiles={() => void chooseContextFiles()}
          onRemoveFile={(path) =>
            setContextFiles((current) =>
              current.filter((item) => item !== path),
            )
          }
          activeFileDisabled={!activeFilePath || activeFileSelected}
          activeFileSelected={activeFileSelected}
          includeTerminal={includeTerminal}
          onIncludeTerminalChange={setIncludeTerminal}
          terminalDetail={
            terminalPrivate
              ? "Unavailable while the active terminal is private"
              : terminalContextAvailable
                ? "Latest visible output from the active terminal"
                : "No terminal output available"
          }
          terminalDisabled={!terminalContextAvailable}
          includeDiff={includeDiff}
          onIncludeDiffChange={setIncludeDiff}
          diffDetail={
            workspaceContextAvailable
              ? "Current unstaged Git diff"
              : "Open a workspace to include Git changes"
          }
          diffDisabled={!workspaceContextAvailable}
        />

        <TaskSkillChips
          skills={skills}
          selected={selectedSkills}
          onToggle={(skillName) =>
            setSelectedSkills((current) =>
              current.includes(skillName)
                ? current.filter((name) => name !== skillName)
                : [...current, skillName],
            )
          }
        />

        <CreateFormActions
          sectioned
          status={error ?? undefined}
          statusTone={error ? "destructive" : "muted"}
          onCancel={() => setViewMode("queue")}
          submitDisabled={!prompt.trim() || dispatching}
          submitLabel={
            <>
              {dispatching ? <Spinner className="size-3" /> : null}
              Run in background
            </>
          }
        />
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
          <SurfaceFilteredEmpty
            message="No tasks match this view."
            onClear={() => {
              setQuery("");
              setFilter("all");
            }}
          />
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
                <TaskRunCard
                  key={task.id}
                  className={index > 0 ? "border-t border-border-subtle" : undefined}
                  title={task.title.replace(/^🤖\s*/, "")}
                  status={status}
                  createdAtMs={task.createdAt}
                  tokens={tokens}
                  subagentCount={run?.subagents.length ?? 0}
                  agentLabel={
                    task.runConfig?.agentId
                      ? (agents.find((agent) => agent.id === task.runConfig?.agentId)
                          ?.name ?? "Custom agent")
                      : undefined
                  }
                  modelLabel={
                    task.runConfig?.modelId
                      ? (MODELS.find((model) => model.id === task.runConfig?.modelId)
                          ?.label ?? task.runConfig.modelId)
                      : undefined
                  }
                  skillsLabel={
                    task.runConfig?.skills?.length
                      ? task.runConfig.skills.join(", ")
                      : undefined
                  }
                  step={run?.step}
                  lastResult={run?.lastResult}
                  outcome={
                    (status === "done" || status === "failed") && run
                      ? {
                          changesCount: run.changes.length,
                          checksPassed: run.verifications.filter(
                            (v) => v.status === "passed",
                          ).length,
                          checksFailed: run.verifications.filter(
                            (v) => v.status === "failed",
                          ).length,
                        }
                      : null
                  }
                  isOpenNow={activeSessionId === task.sessionId}
                  active={active}
                  busyRetry={dispatching}
                  onOpen={() => {
                    switchSession(task.sessionId);
                    onClose();
                  }}
                  onReuse={() => reuseTask(task)}
                  onRetry={
                    status === "failed" ? () => void retryTask(task) : undefined
                  }
                  onStop={active ? () => void cancel(task.id) : undefined}
                  onRemove={
                    active ? undefined : () => setRemoveTarget(task)
                  }
                />
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


