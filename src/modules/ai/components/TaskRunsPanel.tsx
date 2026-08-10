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
  CreateFormActions,
  PromptEditorSection,
  SurfaceFilteredEmpty,
  SurfaceFilterToolbar,
  SurfaceListGroup,
  SurfaceLoadingState,
  SurfacePrimaryAction,
  SurfaceSecondaryAction,
  TaskContextSources,
  TaskRunCard,
  TaskRunConfigSection,
  TaskSkillChips,
  partitionTasksByGroupStatus,
  resolveAssignmentStatusFromRun,
  taskFilterCounts,
  taskMatchesListFilter,
  taskMatchesQuery,
  TASK_PROMPT_TEMPLATES,
  canCreateTaskDraft,
  taskTitleFromPrompt,
  filterEnabledAgents,
  filterTaskSourceAssignments,
  sortByCreatedAtDesc,
  normalizeDialogPathSelection,
  appendUniqueContextPaths,
  stripTaskBotTitlePrefix,
  findCatalogEntryById,
  catalogModelLabel,
  taskRunOutcomeCounts,
  sumRunTokens,
  isActiveFileInContext,
  isTerminalContextAvailable,
  isWorkspaceContextAvailable,
  skillsListLabel,
  catalogEntryName,
  toggleTaskSkillSelection,
  wrapContextFileBlock,
  wrapTerminalContextBlock,
  wrapWorkingTreeDiffBlock,
  composePromptWithSelectedContext,
  removeListValue,
  terminalContextDetailLabel,
  gitDiffContextDetailLabel,
  taskQueueSurfaceSubtitle,
  TASK_CREATE_SURFACE_SUBTITLE,
  taskPermissionModeShortLabel,
  TASK_RUNS_LOADING_LABEL,
  TASK_RUNS_EMPTY_TITLE,
  TASK_RUNS_EMPTY_DESCRIPTION,
  TASK_RUNS_EMPTY_ACTION_LABEL,
  TASK_RUNS_FILTERED_EMPTY_MESSAGE,
  TASK_RUNS_SEARCH_PLACEHOLDER,
  TASK_RUNS_TABS_LABEL,
  TASK_RUNS_SURFACE_EYEBROW,
  TASK_RUNS_CREATE_TITLE,
  TASK_RUNS_CREATE_DESCRIPTION,
  TASK_RUNS_PROMPT_PLACEHOLDER,
  TASK_RUNS_ADD_FILES_DIALOG_TITLE,
  TASK_RUNS_CUSTOM_AGENT_LABEL,
  TASK_RUNS_START_ERROR,
  TASK_RUNS_RETRY_ERROR,
  TASK_RUNS_FILTER_TAB_ATTENTION,
  TASK_RUNS_FILTER_TAB_HISTORY,
  TASK_RUNS_FILTER_TAB_LIVE,
  TASK_RUNS_FILTER_TAB_ALL,
} from "@altai/agent-ui";

const TERMINAL: AssignmentStatus[] = ["done", "failed", "cancelled"];
type TaskFilter = "all" | "active" | "attention" | "finished";

function currentStatus(
  assignment: Assignment,
  run: ReturnType<typeof useAgentRunsStore.getState>["runs"][string] | undefined,
): AssignmentStatus {
  return resolveAssignmentStatusFromRun(
    assignment.status,
    run,
    TERMINAL,
  ) as AssignmentStatus;
}

/**
 * A workspace-level task launcher. Each run has a dedicated chat_id, so a
 * long-running job never steals the current conversation or its context.
 */
export function TaskRunsPanel({
  onClose,
  navigation,
  presentation = "overlay",
  surfaceTitle = "Work",
  surfaceEyebrow = TASK_RUNS_SURFACE_EYEBROW,
}: {
  onClose?: () => void;
  navigation?: ReactNode;
  presentation?: "overlay" | "embedded";
  /** Header title when not using overlay chrome defaults. */
  surfaceTitle?: string;
  surfaceEyebrow?: string;
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
    return filterEnabledAgents(store.all(), (id) => store.isDisabled(id));
  }, [activeAgentId]);

  const tasks = useMemo(
    () => filterTaskSourceAssignments(assignments),
    [assignments],
  );
  const resolvedTasks = useMemo(
    () =>
      sortByCreatedAtDesc(
        tasks.map((task) => ({
          task,
          status: currentStatus(task, runs[task.sessionId]),
          createdAt: task.createdAt,
        })),
      ),
    [runs, tasks],
  );
  const filterCounts = useMemo(
    () =>
      taskFilterCounts(resolvedTasks, ACTIVE_ASSIGNMENT_STATES, TERMINAL),
    [resolvedTasks],
  );
  const visibleTasks = useMemo(() => {
    return resolvedTasks.filter(({ task, status }) => {
      if (
        !taskMatchesListFilter(
          status,
          filter,
          ACTIVE_ASSIGNMENT_STATES,
          TERMINAL,
        )
      ) {
        return false;
      }
      const run = runs[task.sessionId];
      return taskMatchesQuery(
        [
          task.title,
          task.source.kind === "task" ? task.source.prompt : "",
          run?.step ?? "",
          run?.lastResult ?? "",
        ],
        query,
      );
    });
  }, [filter, query, resolvedTasks, runs]);
  const taskGroups = useMemo(
    () => partitionTasksByGroupStatus(visibleTasks),
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
        title: taskTitleFromPrompt(prompt),
        prompt: taskPrompt,
        runConfig: { agentId, modelId, permissionMode, skills: selectedSkills },
      });
      setPrompt("");
      setContextFiles([]);
      setIncludeTerminal(false);
      setIncludeDiff(false);
      setViewMode("queue");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : TASK_RUNS_START_ERROR);
    }
  }

  const reuseTask = (task: Assignment) => {
    if (task.source.kind !== "task") return;
    setPrompt(task.source.prompt);
    if (task.runConfig?.agentId) setAgentId(task.runConfig.agentId);
    if (task.runConfig?.modelId) {
      const knownModel = findCatalogEntryById(MODELS, task.runConfig.modelId);
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
      title: TASK_RUNS_ADD_FILES_DIALOG_TITLE,
    });
    const paths = normalizeDialogPathSelection(selected);
    if (!paths.length) return;
    setContextFiles((current) => appendUniqueContextPaths(current, paths));
  };

  const retryTask = async (task: Assignment) => {
    if (task.source.kind !== "task" || dispatching) return;
    setError(null);
    try {
      await runTask({
        title: stripTaskBotTitlePrefix(task.title),
        prompt: task.source.prompt,
        runConfig: task.runConfig,
      });
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : TASK_RUNS_RETRY_ERROR);
    }
  };

  const liveContext = useChatStore.getState().live;
  const activeFilePath = liveContext.getActiveFile();
  const activeFileSelected = isActiveFileInContext(
    activeFilePath,
    contextFiles,
  );
  const terminalPrivate = liveContext.isActiveTerminalPrivate();
  const terminalContextAvailable = isTerminalContextAvailable(
    terminalPrivate,
    liveContext.getTerminalContext(),
  );
  const workspaceContextAvailable = isWorkspaceContextAvailable(
    liveContext.getCwd(),
    liveContext.getWorkspaceRoot(),
  );

  return (
    <AuxiliarySurface
      title={surfaceTitle}
      eyebrow={surfaceEyebrow}
      icon={Notebook01Icon}
      presentation={presentation}
      subtitle={
        viewMode === "queue"
          ? taskQueueSurfaceSubtitle(
              filterCounts.active,
              filterCounts.attention,
            )
          : TASK_CREATE_SURFACE_SUBTITLE
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
          searchPlaceholder={TASK_RUNS_SEARCH_PLACEHOLDER}
          tabsLabel={TASK_RUNS_TABS_LABEL}
          tabValue={filter}
          onTabChange={(value) => setFilter(value as TaskFilter)}
          tabs={[
            { id: "all", label: TASK_RUNS_FILTER_TAB_ALL, count: filterCounts.all },
            { id: "active", label: TASK_RUNS_FILTER_TAB_LIVE, count: filterCounts.active },
            {
              id: "attention",
              label: TASK_RUNS_FILTER_TAB_ATTENTION,
              count: filterCounts.attention,
            },
            {
              id: "finished",
              label: TASK_RUNS_FILTER_TAB_HISTORY,
              count: filterCounts.finished,
            },
          ]}
        />
      ) : null}
      {viewMode === "create" ? (
      <form onSubmit={start} className="min-h-0 flex-1 overflow-y-auto">
        <PromptEditorSection
          title={TASK_RUNS_CREATE_TITLE}
          description={TASK_RUNS_CREATE_DESCRIPTION}
          value={prompt}
          onChange={setPrompt}
          textareaId="background-task-prompt"
          placeholder={TASK_RUNS_PROMPT_PLACEHOLDER}
          templates={TASK_PROMPT_TEMPLATES.map((template) => ({
            label: template.label,
            value: template.prompt,
          }))}
        />
        <TaskRunConfigSection>
          <DropdownMenu>
            <DropdownMenuTrigger className="flex h-6 items-center gap-1 rounded-md border border-border bg-card px-2 text-[10px] text-muted-foreground transition-colors hover:bg-foreground/[0.055]">
              {catalogEntryName(agents, agentId, "Default")}
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
              <span className="truncate">
                {taskPermissionModeShortLabel(permissionMode)}
              </span>
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
                appendUniqueContextPaths(current, [path]),
              );
            }
          }}
          onChooseFiles={() => void chooseContextFiles()}
          onRemoveFile={(path) =>
            setContextFiles((current) => removeListValue(current, path))
          }
          activeFileDisabled={!activeFilePath || activeFileSelected}
          activeFileSelected={activeFileSelected}
          includeTerminal={includeTerminal}
          onIncludeTerminalChange={setIncludeTerminal}
          terminalDetail={terminalContextDetailLabel({
            terminalPrivate,
            terminalAvailable: terminalContextAvailable,
          })}
          terminalDisabled={!terminalContextAvailable}
          includeDiff={includeDiff}
          onIncludeDiffChange={setIncludeDiff}
          diffDetail={gitDiffContextDetailLabel(workspaceContextAvailable)}
          diffDisabled={!workspaceContextAvailable}
        />

        <TaskSkillChips
          skills={skills}
          selected={selectedSkills}
          onToggle={(skillName) =>
            setSelectedSkills((current) =>
              toggleTaskSkillSelection(current, skillName),
            )
          }
        />

        <CreateFormActions
          sectioned
          status={error ?? undefined}
          statusTone={error ? "destructive" : "muted"}
          onCancel={() => setViewMode("queue")}
          submitDisabled={!canCreateTaskDraft({ prompt, creating: dispatching })}
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
          <SurfaceLoadingState density="panel">
            <Spinner className="size-3.5" /> {TASK_RUNS_LOADING_LABEL}
          </SurfaceLoadingState>
        ) : tasks.length === 0 ? (
          <SurfaceEmptyState
            icon={Notebook01Icon}
            title={TASK_RUNS_EMPTY_TITLE}
            description={TASK_RUNS_EMPTY_DESCRIPTION}
            action={
              <button
                type="button"
                onClick={() => setViewMode("create")}
                className="rounded-md bg-primary px-3 py-1.5 text-[10px] font-semibold text-primary-foreground"
              >
                {TASK_RUNS_EMPTY_ACTION_LABEL}
              </button>
            }
          />
        ) : visibleTasks.length === 0 ? (
          <SurfaceFilteredEmpty
            message={TASK_RUNS_FILTERED_EMPTY_MESSAGE}
            onClear={() => {
              setQuery("");
              setFilter("all");
            }}
          />
        ) : (
          <div className="space-y-5">
            {taskGroups.map((group) => (
              <SurfaceListGroup
                key={group.id}
                title={group.title}
                description={group.description}
                count={group.items.length}
              >
            {group.items.map(({ task, status }, index) => {
              const run = runs[task.sessionId];
              const active = ACTIVE_ASSIGNMENT_STATES.includes(status);
              return (
                <TaskRunCard
                  key={task.id}
                  className={index > 0 ? "border-t border-border-subtle" : undefined}
                  title={stripTaskBotTitlePrefix(task.title)}
                  status={status}
                  createdAtMs={task.createdAt}
                  tokens={sumRunTokens(run?.tokens)}
                  subagentCount={run?.subagents.length ?? 0}
                  agentLabel={catalogEntryName(
                    agents,
                    task.runConfig?.agentId,
                    TASK_RUNS_CUSTOM_AGENT_LABEL,
                  )}
                  modelLabel={catalogModelLabel(
                    MODELS,
                    task.runConfig?.modelId,
                  )}
                  skillsLabel={skillsListLabel(task.runConfig?.skills)}
                  step={run?.step}
                  lastResult={run?.lastResult}
                  outcome={
                    (status === "done" || status === "failed") && run
                      ? taskRunOutcomeCounts({
                          changesCount: run.changes.length,
                          verifications: run.verifications,
                        })
                      : null
                  }
                  isOpenNow={activeSessionId === task.sessionId}
                  active={active}
                  busyRetry={dispatching}
                  onOpen={() => {
                    switchSession(task.sessionId);
                    onClose?.();
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
              </SurfaceListGroup>
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
            {removeTarget ? stripTaskBotTitlePrefix(removeTarget.title) : null}
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
        blocks.push(wrapContextFileBlock(path, result.content));
      }
    } catch {
      /* unavailable files simply stay out of the task */
    }
  }
  if (selected.terminal && !live.isActiveTerminalPrivate()) {
    const output = live.getTerminalContext();
    if (output) {
      const block = wrapTerminalContextBlock(output);
      if (block) blocks.push(block);
    }
  }
  if (selected.diff) {
    const cwd = live.getCwd() ?? live.getWorkspaceRoot();
    if (cwd) {
      try {
        const repo = await native.gitResolveRepo(cwd);
        if (repo) {
          const diff = await native.gitDiff(repo.repoRoot, null, false);
          const block = wrapWorkingTreeDiffBlock(
            diff.diffText,
            Boolean(diff.truncated),
          );
          if (block) blocks.push(block);
        }
      } catch { /* non-git workspaces have no diff to include */ }
    }
  }
  return composePromptWithSelectedContext(prompt, blocks);
}

