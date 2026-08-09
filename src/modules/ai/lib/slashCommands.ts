import {
  Archive02Icon,
  CalendarSyncIcon,
  CheckListIcon,
  File01Icon,
  Notification01Icon,
  Search01Icon,
  ShieldUserIcon,
  SparklesIcon,
} from "@hugeicons/core-free-icons";
import { invoke } from "@tauri-apps/api/core";
import { openSettingsWindow } from "@/modules/settings/openSettingsWindow";
import { currentWorkspaceFolder } from "@/modules/workspace/folder";
import {
  ALTAI_CMD_RE,
  appendSlashCommandFocus,
  findAgentByIdOrName,
  hasSlashCommandTail,
  isPlanModeOffTail,
  planModeOffToast,
  planModeToggleToast,
  startedNewChatToast,
  openedChatSessionsToast,
  renameUsageToast,
  renamedActiveChatToast,
  retryingLastRequestToast,
  cancellationRequestedToast,
  compactionRequestedToast,
  openedRunDetailsToast,
  openedChangeReviewToast,

  agentSettingsToast,
  switchedAgentToast,
  parseComposerSlashLead,
  filterSlashCommands as filterSlashCommandsShared,
  isWorkspaceSlashCommandPath,
  joinWorkspaceRelativePath,
  parseWorkspaceWorkflowCommand as parseWorkspaceWorkflowCommandShared,
  resolveSlashCommandInIndex as resolveSlashCommandInIndexShared,
  wrapWithCommandMarker,
} from "@altai/agent-ui";
import { native } from "./native";
import { useAgentsStore } from "../store/agentsStore";
import { retryLastMessage, requestStop, useChatStore } from "../store/chatStore";
import { usePlanStore } from "../store/planStore";

export { wrapWithCommandMarker, ALTAI_CMD_RE };

/**
 * Outcome of intercepting a slash command from the composer.
 *
 * - `handled`: command completed locally; the composer must not send a turn.
 * - `send-prompt`: command expands into a normal, approval-gated agent turn.
 * - `none`: not a known command; let the composer behave as usual.
 */
export type SlashOutcome =
  | { kind: "handled"; toast?: string }
  | { kind: "send-prompt"; prompt: string; commandName?: string }
  | { kind: "none" };

export type SlashCommandCategory =
  | "session"
  | "workspace"
  | "code"
  | "quality"
  | "project"
  | "settings";

export type SlashCommandBehavior = "action" | "prompt" | "workflow";
export type SlashCommandSource = "builtin" | "workspace";

export type SlashCommandMeta = {
  name: string;
  invocation: string;
  label: string;
  description: string;
  aliases?: readonly string[];
  category: SlashCommandCategory;
  behavior: SlashCommandBehavior;
  source: SlashCommandSource;
  icon: typeof SparklesIcon;
  workflowPath?: string;
  workflowInstructions?: string;
};

type BuiltinCommand = Omit<SlashCommandMeta, "source">;

const builtin = (command: BuiltinCommand): SlashCommandMeta => ({
  ...command,
  source: "builtin",
});

const INIT_PROMPT = `Scan this workspace and produce ALTAI.md at the workspace root with:

- One-paragraph project description.
- Build / test / dev commands.
- Architecture overview (subsystems, data flow, key dirs).
- Conventions worth knowing (naming, patterns, gotchas).
- Paths to entry points.

Use grep/glob/list_directory/read_file to explore. Cap ALTAI.md under 200 lines. Use write_file to create it (will go through normal approval).`;

const BUILTIN_COMMANDS: readonly SlashCommandMeta[] = [
  builtin({ name: "new", invocation: "/new", label: "New chat", description: "Start a fresh task session.", aliases: ["clear"], category: "session", behavior: "action", icon: SparklesIcon }),
  builtin({ name: "sessions", invocation: "/sessions", label: "Chat sessions", description: "Open recent tasks and session history.", aliases: ["history", "resume"], category: "session", behavior: "action", icon: File01Icon }),
  builtin({ name: "rename", invocation: "/rename", label: "Rename chat", description: "Rename the active chat. Add the new title after the command.", category: "session", behavior: "action", icon: File01Icon }),
  builtin({ name: "retry", invocation: "/retry", label: "Retry last turn", description: "Rewind and rerun the latest user request.", aliases: ["regenerate"], category: "session", behavior: "action", icon: SparklesIcon }),
  builtin({ name: "stop", invocation: "/stop", label: "Stop agent", description: "Request cancellation of the active agent run.", aliases: ["cancel"], category: "session", behavior: "action", icon: Archive02Icon }),
  builtin({ name: "compact", invocation: "/compact", label: "Compact context", description: "Summarize older conversation context and keep the active task focused.", aliases: ["smol", "condense", "summarize"], category: "session", behavior: "action", icon: Archive02Icon }),
  builtin({ name: "init", invocation: "/init", label: "Initialize workspace", description: "Scan the workspace and draft an ALTAI.md project guide.", category: "workspace", behavior: "prompt", icon: SparklesIcon }),
  builtin({ name: "index", invocation: "/index", label: "Map codebase", description: "Create a concise codebase map with entry points, boundaries, and commands.", aliases: ["map"], category: "workspace", behavior: "prompt", icon: Search01Icon }),
  builtin({ name: "search", invocation: "/search", label: "Search workspace", description: "Find code, configuration, or behaviour across the workspace.", aliases: ["find"], category: "workspace", behavior: "prompt", icon: Search01Icon }),
  builtin({ name: "status", invocation: "/status", label: "Run details", description: "Open activity and details for the current run.", aliases: ["activity", "inspect"], category: "workspace", behavior: "action", icon: SparklesIcon }),
  builtin({ name: "git-status", invocation: "/git-status", label: "Git status", description: "Inspect changed files, branch state, and a concise next-step summary.", aliases: ["git"], category: "workspace", behavior: "prompt", icon: File01Icon }),
  builtin({ name: "diff", invocation: "/diff", label: "Review diff", description: "Inspect working-tree changes and explain risks before editing further.", category: "workspace", behavior: "prompt", icon: File01Icon }),
  builtin({ name: "plan", invocation: "/plan", label: "Plan mode", description: "Toggle plan-first mode before making changes. Use “off” to exit.", aliases: ["architect"], category: "code", behavior: "action", icon: CheckListIcon }),
  builtin({ name: "explain", invocation: "/explain", label: "Explain code", description: "Explain a file, selection, subsystem, or behaviour without changing it.", aliases: ["ask"], category: "code", behavior: "prompt", icon: SparklesIcon }),
  builtin({ name: "fix", invocation: "/fix", label: "Fix issue", description: "Investigate a problem, make the smallest safe fix, and verify it.", aliases: ["debug"], category: "code", behavior: "prompt", icon: SparklesIcon }),
  builtin({ name: "refactor", invocation: "/refactor", label: "Refactor", description: "Improve structure while preserving behaviour and checking the result.", category: "code", behavior: "prompt", icon: SparklesIcon }),
  builtin({ name: "todo", invocation: "/todo", label: "Create task plan", description: "Turn a goal into a tracked, ordered implementation checklist.", aliases: ["checklist"], category: "code", behavior: "prompt", icon: CheckListIcon }),
  builtin({ name: "test", invocation: "/test", label: "Run tests", description: "Discover the relevant test command, run it, and report failures with evidence.", category: "quality", behavior: "prompt", icon: CheckListIcon }),
  builtin({ name: "lint", invocation: "/lint", label: "Run lint", description: "Find and run the project lint command, then fix actionable issues.", category: "quality", behavior: "prompt", icon: CheckListIcon }),
  builtin({ name: "build", invocation: "/build", label: "Build project", description: "Run the production build and diagnose any failures.", category: "quality", behavior: "prompt", icon: CheckListIcon }),
  builtin({ name: "review", invocation: "/review", label: "Review changes", description: "Open the change review surface, or ask for a focused code review with a scope.", category: "quality", behavior: "action", icon: ShieldUserIcon }),
  builtin({ name: "security", invocation: "/security", label: "Security review", description: "Audit the requested scope for security issues and verify findings.", category: "quality", behavior: "prompt", icon: ShieldUserIcon }),
  builtin({ name: "perf", invocation: "/perf", label: "Performance review", description: "Find likely performance bottlenecks and propose measurable improvements.", aliases: ["performance"], category: "quality", behavior: "prompt", icon: SparklesIcon }),
  builtin({ name: "docs", invocation: "/docs", label: "Update documentation", description: "Inspect the change or scope and update the relevant documentation.", aliases: ["document"], category: "project", behavior: "prompt", icon: File01Icon }),
  builtin({ name: "workflow", invocation: "/workflow", label: "Create workflow", description: "Design or update a reusable WORKFLOW.md automation process.", category: "project", behavior: "prompt", icon: CalendarSyncIcon }),
  builtin({ name: "research", invocation: "/research", label: "Research", description: "Research the question with primary sources and return cited findings.", category: "project", behavior: "prompt", icon: Search01Icon }),
  builtin({ name: "paper", invocation: "/paper", label: "Import arXiv paper", description: "Attach an arXiv paper as task context.", category: "project", behavior: "action", icon: File01Icon }),
  builtin({ name: "tasks", invocation: "/tasks", label: "Work", description: "Open Operations work (active, completed, and review-ready runs).", aliases: ["work"], category: "project", behavior: "action", icon: CheckListIcon }),
  builtin({ name: "inbox", invocation: "/inbox", label: "Notifications", description: "Open the Operations inbox for agent attention items.", category: "project", behavior: "action", icon: Notification01Icon }),
  builtin({ name: "automations", invocation: "/automations", label: "Scheduled", description: "Open Operations scheduled agent work.", aliases: ["schedule"], category: "project", behavior: "action", icon: CalendarSyncIcon }),
  builtin({ name: "agents", invocation: "/agents", label: "Agent settings", description: "Open agent selection and custom-agent settings. Add an agent name to switch directly.", aliases: ["agent"], category: "settings", behavior: "action", icon: SparklesIcon }),
  builtin({ name: "models", invocation: "/models", label: "Model settings", description: "Open model selection and provider settings.", aliases: ["model"], category: "settings", behavior: "action", icon: SparklesIcon }),
  builtin({ name: "permissions", invocation: "/permissions", label: "Permissions", description: "Open agent permission controls and safety settings.", aliases: ["permission"], category: "settings", behavior: "action", icon: ShieldUserIcon }),
  builtin({ name: "mcp", invocation: "/mcp", label: "MCP settings", description: "Open connected-tool and MCP server settings.", aliases: ["mcps"], category: "settings", behavior: "action", icon: SparklesIcon }),
  builtin({ name: "skills", invocation: "/skills", label: "Skills", description: "Open installed skills and add reusable workflows.", category: "settings", behavior: "action", icon: SparklesIcon }),
  builtin({ name: "context", invocation: "/context", label: "Context settings", description: "Open context, compaction, and project-instruction settings.", category: "settings", behavior: "action", icon: Archive02Icon }),
];

/** Canonical built-in registry. Dynamic workspace commands are included by
 * `getSlashCommandIndex()` and never overwrite these stable names. */
export const SLASH_COMMANDS: Record<string, SlashCommandMeta> = Object.fromEntries(
  BUILTIN_COMMANDS.map((command) => [command.name, command]),
);
export const SLASH_COMMAND_INDEX = Object.freeze(BUILTIN_COMMANDS);

let workspaceCommands: readonly SlashCommandMeta[] = [];
let indexedWorkspace: string | null = null;

export function getSlashCommandIndex(): readonly SlashCommandMeta[] {
  return [...SLASH_COMMAND_INDEX, ...workspaceCommands];
}

export function findSlashCommands(query = ""): readonly SlashCommandMeta[] {
  return filterSlashCommandsShared(getSlashCommandIndex(), query);
}

export function resolveSlashCommand(name: string): SlashCommandMeta | undefined {
  return resolveSlashCommandInIndexShared(getSlashCommandIndex(), name);
}

/** Refresh workspace-defined commands from ALTAI's own command directory.
 * Command files are prompt content only; they can never bypass normal agent
 * approval rules. */
export async function refreshWorkspaceSlashCommands(
  workspaceRoot: string | null,
): Promise<readonly SlashCommandMeta[]> {
  indexedWorkspace = workspaceRoot;
  workspaceCommands = [];
  if (!workspaceRoot) return workspaceCommands;

  try {
    const listed = await native.listWorkspaceFiles(workspaceRoot, {
      showHidden: true,
      maxDepth: 8,
      limit: 10_000,
    });
    const files = listed.files
      .filter((path) => isWorkspaceSlashCommandPath(path))
      .sort((a, b) => a.localeCompare(b));
    const builtins = new Set(
      SLASH_COMMAND_INDEX.flatMap((command) => [command.name, ...(command.aliases ?? [])]),
    );
    const discovered = await Promise.all(
      files.map(async (relativePath) => {
        const result = await native.readFile(joinWorkspaceRelativePath(workspaceRoot, relativePath), {
          enforceIsanagentignore: true,
        });
        return result.kind === "text"
          ? parseWorkflowCommand(relativePath, result.content)
          : null;
      }),
    );
    const usedNames = new Set(builtins);
    const accepted: SlashCommandMeta[] = [];
    for (const command of discovered) {
      if (!command) continue;
      const names = [command.name, ...(command.aliases ?? [])];
      if (names.some((name) => usedNames.has(name))) continue;
      names.forEach((name) => usedNames.add(name));
      accepted.push(command);
    }

    // Ignore results from a workspace that was switched while reads were in
    // flight. The next effect refreshes that workspace independently.
    if (indexedWorkspace !== workspaceRoot) return workspaceCommands;
    workspaceCommands = accepted;
  } catch (error) {
    console.warn("Could not index workspace slash commands", error);
  }
  return workspaceCommands;
}

function parseWorkflowCommand(path: string, source: string): SlashCommandMeta | null {
  const parsed = parseWorkspaceWorkflowCommandShared(path, source);
  if (!parsed) return null;
  return {
    ...parsed,
    icon: CalendarSyncIcon,
  };
}

export function tryRunSlashCommand(input: string): SlashOutcome {
  const leadParse = parseComposerSlashLead(input);
  if (!leadParse) return { kind: "none" };
  const { head, tail } = leadParse;
  const command = resolveSlashCommand(head);
  if (!command) return { kind: "none" };

  if (command.behavior === "workflow") {
    return {
      kind: "send-prompt",
      commandName: command.name,
      prompt: `${command.workflowInstructions}\n\n${tail ? `Task input: ${tail}` : "Follow this workflow for the current workspace."}`,
    };
  }
  if (command.behavior === "prompt") {
    return {
      kind: "send-prompt",
      commandName: command.name,
      prompt: promptFor(command.name, tail),
    };
  }
  return runLocalCommand(command.name, tail);
}

function runLocalCommand(name: string, tail: string): SlashOutcome {
  const chat = useChatStore.getState();
  switch (name) {
    case "new":
      chat.newSession();
      return { kind: "handled", toast: startedNewChatToast() };
    case "sessions":
      openAiSurface("history");
      return { kind: "handled", toast: openedChatSessionsToast() };
    case "rename":
      if (!hasSlashCommandTail(tail) || !chat.activeSessionId) {
        return { kind: "handled", toast: renameUsageToast() };
      }
      chat.renameSession(chat.activeSessionId, tail);
      return { kind: "handled", toast: renamedActiveChatToast() };
    case "retry":
      void retryLastMessage();
      return { kind: "handled", toast: retryingLastRequestToast() };
    case "stop":
      void requestStop(chat.activeSessionId);
      return { kind: "handled", toast: cancellationRequestedToast() };
    case "compact":
      void runCompactNow(tail || undefined);
      return { kind: "handled", toast: compactionRequestedToast() };
    case "status":
      openAiSurface("inspector");
      return { kind: "handled", toast: openedRunDetailsToast() };
    case "plan": {
      const store = usePlanStore.getState();
      if (isPlanModeOffTail(tail)) {
        store.disable();
        return { kind: "handled", toast: planModeOffToast() };
      }
      store.toggle();
      return {
        kind: "handled",
        toast: planModeToggleToast(usePlanStore.getState().active),
      };
    }
    case "paper":
      chat.setPaperImportOpen(true);
      return { kind: "handled" };
    case "review":
      if (tail) return { kind: "send-prompt", commandName: name, prompt: promptFor(name, tail) };
      openAiSurface("review");
      return { kind: "handled", toast: openedChangeReviewToast() };
    case "tasks":
      openOperationsSurface("work", "runs");
      return { kind: "handled", toast: "Opened Operations work" };
    case "inbox":
      openOperationsSurface("inbox");
      return { kind: "handled", toast: "Opened Operations inbox" };
    case "automations":
      openOperationsSurface("work", "scheduled");
      return { kind: "handled", toast: "Opened Operations scheduled work" };
    case "agents": {
      if (tail) {
        const agent = findAgentByIdOrName(
          useAgentsStore.getState().enabled(),
          tail,
        );
        if (agent) {
          useAgentsStore.getState().setActiveId(agent.id);
          return { kind: "handled", toast: switchedAgentToast(agent.name) };
        }
      }
      openSettingsWindow("agents");
      return {
        kind: "handled",
        toast: agentSettingsToast(Boolean(tail.trim())),
      };
    }
    case "models":
      openSettingsWindow("models");
      return { kind: "handled", toast: "Opened model settings" };
    case "permissions":
      openSettingsWindow("general");
      return { kind: "handled", toast: "Opened permission settings" };
    case "mcp":
      openSettingsWindow("mcp");
      return { kind: "handled", toast: "Opened MCP settings" };
    case "skills":
      openSettingsWindow("skills");
      return { kind: "handled", toast: "Opened skills" };
    case "context":
      openSettingsWindow("context");
      return { kind: "handled", toast: "Opened context settings" };
    default:
      return { kind: "none" };
  }
}

function openAiSurface(
  surface: "history" | "inspector" | "work" | "inbox" | "review",
  view?: "runs" | "scheduled",
): void {
  if (typeof window === "undefined") return;
  window.dispatchEvent(new CustomEvent("altai:open-ai-surface", { detail: { surface, view } }));
}

/** Open the canonical Operations tab on a live secondary route. */
function openOperationsSurface(
  view: "overview" | "work" | "runs" | "inbox",
  workHubView?: "runs" | "scheduled",
): void {
  if (typeof window === "undefined") return;
  window.dispatchEvent(
    new CustomEvent("altai:open-operations", {
      detail: { view, workHubView },
    }),
  );
}

function promptFor(name: string, tail: string): string {
  const prompts: Record<string, string> = {
    init: INIT_PROMPT,
    index: "Inspect this workspace without changing files. Produce a compact codebase map: entry points, major modules, data flow, build/test commands, conventions, and high-risk areas. Cite concrete paths for each conclusion.",
    search: "Search the workspace for the requested concept. Report the most relevant paths and lines, explain how they connect, and do not make changes unless explicitly asked.",
    "git-status": "Inspect the Git repository state. Summarize branch/upstream, changed and untracked files, staged versus unstaged work, and the safest next step. Do not modify Git state.",
    diff: "Inspect the current working-tree diff. Summarize intent, affected areas, likely regressions, and missing verification. Do not apply changes.",
    explain: "Explain the requested code or behaviour accurately. Read the relevant workspace files first, cite paths, and do not change files.",
    fix: "Investigate the reported issue first. Identify the root cause, make the smallest focused fix, then run the most relevant verification and report evidence.",
    refactor: "Inspect the requested scope and existing conventions. Propose a focused refactor, preserve behaviour, make changes only after understanding dependencies, and verify the result.",
    todo: "Break this task into an ordered, concrete checklist using the todo tool. Include discovery, implementation, verification, and any approval boundary.",
    test: "Discover the project’s relevant test command from its configuration and documentation. Run the smallest relevant test scope first, diagnose failures, and report exact results.",
    lint: "Discover the project lint command, run it for the relevant scope, fix clear issues when appropriate, and report the final command result.",
    build: "Discover the production build command, run it, diagnose failures if any, and report the exact verification result.",
    review: "Review the requested change scope for correctness, regressions, maintainability, and missing tests. Read the diff and surrounding code; do not modify files unless explicitly asked.",
    security: "Perform a focused security review of the requested scope. Look for auth, authorization, injection, data exposure, dependency, and unsafe execution issues. Report only evidence-backed findings with paths and severity.",
    perf: "Review the requested scope for measurable performance risks. Inspect hot paths, rendering, I/O, network, and algorithmic complexity; propose changes with expected impact and verification.",
    docs: "Inspect the requested feature or change and update the documentation that users or maintainers need. Keep claims tied to the actual implementation and verify links/commands where possible.",
    workflow: "Inspect existing project automation and WORKFLOW.md. Propose or update a reusable workflow with clear trigger, steps, validation, approval boundaries, and rollback notes.",
    research: "Research the requested topic using primary, current sources where possible. Separate facts from inference, cite sources, and translate findings into concrete project implications.",
  };
  return appendSlashCommandFocus(
    prompts[name] ?? "Handle the requested task carefully and verify the result.",
    tail,
  );
}

/** Fire a manual `/compact` directly (no input prefill, no Enter required). */
export async function runCompactNow(focusInstructions?: string): Promise<boolean> {
  const store = useChatStore.getState();
  const chatId = store.activeSessionId;
  const workspacePath = currentWorkspaceFolder();
  if (!chatId || !workspacePath) return false;
  try {
    await invoke("agent_compact", {
      workspacePath,
      chatId,
      focusInstructions: focusInstructions?.trim() || null,
    });
    store.addActivity({
      label: "Context compaction requested",
      detail: "Queued directly on the agent runtime",
      kind: "agent",
      tone: "success",
    });
    return true;
  } catch (error) {
    store.addActivity({
      label: "Context compaction failed",
      detail: error instanceof Error ? error.message : String(error),
      kind: "agent",
      tone: "error",
    });
    return false;
  }
}
