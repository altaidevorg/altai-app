/**
 * Host-neutral slash command registry for the Chat composer (A6.97).
 * Outcomes are pure; VS Code/Desktop hosts dispatch host actions.
 */

import { promptForSlashCommand } from "./slashCommandPrompt.js";

export type SlashCommandCategory =
  | "session"
  | "workspace"
  | "code"
  | "quality"
  | "project"
  | "settings";

export type SlashCommandBehavior = "action" | "prompt";

export type SlashCommandMeta = {
  name: string;
  invocation: string;
  label: string;
  description: string;
  aliases?: readonly string[];
  category: SlashCommandCategory;
  behavior: SlashCommandBehavior;
};

export type SlashHostAction =
  | "new"
  | "sessions"
  | "rename"
  | "retry"
  | "stop"
  | "compact"
  | "status"
  | "plan"
  | "review"
  | "tasks"
  | "inbox"
  | "automations"
  | "models"
  | "permissions"
  | "mcp"
  | "skills"
  | "settings"
  | "help"
  | "logs"
  | "diagnostics"
  | "restart-host"
  | "new-task"
  | "new-automation"
  | "version"
  | "copy"
  | "connect"
  | "disconnect"
  | "attach-diff"
  | "attach-terminal"
  | "attach-file"
  | "attach-selection"
  | "walkthrough"
  | "extension-settings"
  | "copy-diag"
  | "attach-problems"
  | "pick-root";

export type SlashOutcome =
  | { kind: "none" }
  | { kind: "handled"; action: SlashHostAction; tail: string; toast?: string }
  | { kind: "send-prompt"; prompt: string; commandName: string };

const COMMANDS: readonly SlashCommandMeta[] = [
  { name: "new", invocation: "/new", label: "New chat", description: "Start a fresh task session.", aliases: ["clear"], category: "session", behavior: "action" },
  { name: "sessions", invocation: "/sessions", label: "Chat sessions", description: "Open recent tasks and session history.", aliases: ["history", "resume"], category: "session", behavior: "action" },
  { name: "rename", invocation: "/rename", label: "Rename chat", description: "Rename the active chat. Add the new title after the command.", category: "session", behavior: "action" },
  { name: "retry", invocation: "/retry", label: "Retry last turn", description: "Rewind and rerun the latest user request.", aliases: ["regenerate"], category: "session", behavior: "action" },
  { name: "stop", invocation: "/stop", label: "Stop agent", description: "Request cancellation of the active agent run.", aliases: ["cancel"], category: "session", behavior: "action" },
  { name: "compact", invocation: "/compact", label: "Compact context", description: "Summarize older conversation context.", aliases: ["smol", "condense", "summarize"], category: "session", behavior: "action" },
  { name: "init", invocation: "/init", label: "Initialize workspace", description: "Scan the workspace and draft an ALTAI.md project guide.", category: "workspace", behavior: "prompt" },
  { name: "index", invocation: "/index", label: "Map codebase", description: "Create a concise codebase map.", aliases: ["map"], category: "workspace", behavior: "prompt" },
  { name: "search", invocation: "/search", label: "Search workspace", description: "Find code or configuration across the workspace.", aliases: ["find"], category: "workspace", behavior: "prompt" },
  { name: "status", invocation: "/status", label: "Run details", description: "Open activity and details for the current run.", aliases: ["activity", "inspect"], category: "workspace", behavior: "action" },
  { name: "git-status", invocation: "/git-status", label: "Git status", description: "Inspect branch and working-tree state.", aliases: ["git"], category: "workspace", behavior: "prompt" },
  { name: "diff", invocation: "/diff", label: "Review diff", description: "Inspect working-tree changes and explain risks.", category: "workspace", behavior: "prompt" },
  { name: "plan", invocation: "/plan", label: "Plan mode", description: "Toggle plan-first mode. Use “off” to exit.", aliases: ["architect"], category: "code", behavior: "action" },
  { name: "explain", invocation: "/explain", label: "Explain code", description: "Explain code or behaviour without changing it.", aliases: ["ask"], category: "code", behavior: "prompt" },
  { name: "fix", invocation: "/fix", label: "Fix issue", description: "Investigate, fix the smallest safe change, and verify.", aliases: ["debug"], category: "code", behavior: "prompt" },
  { name: "refactor", invocation: "/refactor", label: "Refactor", description: "Improve structure while preserving behaviour.", category: "code", behavior: "prompt" },
  { name: "todo", invocation: "/todo", label: "Create task plan", description: "Turn a goal into an ordered checklist.", aliases: ["checklist"], category: "code", behavior: "prompt" },
  { name: "test", invocation: "/test", label: "Run tests", description: "Discover and run the relevant test command.", category: "quality", behavior: "prompt" },
  { name: "lint", invocation: "/lint", label: "Run lint", description: "Find/run project lint and fix actionable issues.", category: "quality", behavior: "prompt" },
  { name: "build", invocation: "/build", label: "Build project", description: "Run the production build and diagnose failures.", category: "quality", behavior: "prompt" },
  { name: "review", invocation: "/review", label: "Review changes", description: "Open change review, or ask for a scoped code review.", category: "quality", behavior: "action" },
  { name: "security", invocation: "/security", label: "Security review", description: "Audit the requested scope for security issues.", category: "quality", behavior: "prompt" },
  { name: "perf", invocation: "/perf", label: "Performance review", description: "Find likely performance bottlenecks.", aliases: ["performance"], category: "quality", behavior: "prompt" },
  { name: "docs", invocation: "/docs", label: "Update documentation", description: "Update docs for the requested change.", aliases: ["document"], category: "project", behavior: "prompt" },
  { name: "workflow", invocation: "/workflow", label: "Create workflow", description: "Design or update a reusable WORKFLOW.md process.", category: "project", behavior: "prompt" },
  { name: "research", invocation: "/research", label: "Research", description: "Research with primary sources and return cited findings.", category: "project", behavior: "prompt" },
  { name: "tasks", invocation: "/tasks", label: "Work", description: "Open Operations work (runs).", aliases: ["work"], category: "project", behavior: "action" },
  { name: "new-task", invocation: "/new-task", label: "New task", description: "Open Operations and start a new background task. Optional title after the command.", aliases: ["task"], category: "project", behavior: "action" },
  { name: "inbox", invocation: "/inbox", label: "Notifications", description: "Open the Operations inbox.", category: "project", behavior: "action" },
  { name: "automations", invocation: "/automations", label: "Scheduled", description: "Open Operations scheduled work.", aliases: ["schedule"], category: "project", behavior: "action" },
  { name: "new-automation", invocation: "/new-automation", label: "New automation", description: "Open Scheduled and start a new automation. Optional title after the command.", aliases: ["schedule-new"], category: "project", behavior: "action" },
  { name: "models", invocation: "/models", label: "Model settings", description: "Open Settings to pick a model.", aliases: ["model"], category: "settings", behavior: "action" },
  { name: "permissions", invocation: "/permissions", label: "Permissions", description: "Open Settings for permission modes.", aliases: ["permission"], category: "settings", behavior: "action" },
  { name: "mcp", invocation: "/mcp", label: "MCP", description: "Open Settings for MCP servers.", aliases: ["mcps"], category: "settings", behavior: "action" },
  { name: "skills", invocation: "/skills", label: "Skills", description: "Open Settings for installed skills.", category: "settings", behavior: "action" },
  { name: "settings", invocation: "/settings", label: "Settings", description: "Open the Settings surface.", aliases: ["config", "prefs"], category: "settings", behavior: "action" },
  { name: "help", invocation: "/help", label: "Help", description: "List available slash commands.", aliases: ["commands", "?"], category: "settings", behavior: "action" },
  { name: "version", invocation: "/version", label: "Version compatibility", description: "Show extension and host protocol pin summary.", aliases: ["compat"], category: "settings", behavior: "action" },
  { name: "copy", invocation: "/copy", label: "Copy chat", description: "Copy the current transcript as plain text.", aliases: ["export"], category: "session", behavior: "action" },
  { name: "connect", invocation: "/connect", label: "Connect provider", description: "Connect an AI provider credential via the Extension Host.", aliases: ["provider"], category: "settings", behavior: "action" },
  { name: "disconnect", invocation: "/disconnect", label: "Clear credential", description: "Remove a stored provider credential via the Extension Host.", aliases: ["clear-credential"], category: "settings", behavior: "action" },
  { name: "walkthrough", invocation: "/walkthrough", label: "Getting started", description: "Open the ALTAI Getting Started walkthrough.", aliases: ["intro", "getting-started"], category: "settings", behavior: "action" },
  { name: "extension-settings", invocation: "/extension-settings", label: "Extension settings", description: "Open VS Code settings filtered to ALTAI (host path, etc.).", aliases: ["ext-settings", "host-path"], category: "settings", behavior: "action" },
  { name: "copy-diag", invocation: "/copy-diag", label: "Copy diagnostics", description: "Copy the ALTAI diagnostics report to the clipboard.", aliases: ["copy-diagnostics"], category: "settings", behavior: "action" },
  { name: "attach-problems", invocation: "/attach-problems", label: "Attach problems", description: "Attach Problems for the active file as composer context.", aliases: ["problems", "errors"], category: "workspace", behavior: "action" },
  { name: "pick-root", invocation: "/pick-root", label: "Pick project root", description: "Choose the multi-root folder for the ALTAI agent host.", aliases: ["root", "project-root"], category: "workspace", behavior: "action" },
  { name: "attach-diff", invocation: "/attach-diff", label: "Attach working tree", description: "Attach the git working-tree summary to the composer.", aliases: ["diff-attach", "wt"], category: "workspace", behavior: "action" },
  { name: "attach-terminal", invocation: "/attach-terminal", label: "Attach terminal", description: "Attach active terminal context to the composer.", aliases: ["terminal", "tty"], category: "workspace", behavior: "action" },
  { name: "attach-file", invocation: "/attach-file", label: "Attach active file", description: "Attach the active editor file URI to the composer.", aliases: ["file", "active-file"], category: "workspace", behavior: "action" },
  { name: "attach-selection", invocation: "/attach-selection", label: "Attach selection", description: "Attach the current editor selection to the composer.", aliases: ["selection", "sel"], category: "workspace", behavior: "action" },
  { name: "logs", invocation: "/logs", label: "Open logs", description: "Show the ALTAI output channel.", category: "settings", behavior: "action" },
  { name: "diagnostics", invocation: "/diagnostics", label: "Run diagnostics", description: "Write host diagnostics to the ALTAI log channel.", aliases: ["diag"], category: "settings", behavior: "action" },
  { name: "restart-host", invocation: "/restart-host", label: "Restart host", description: "Restart the ALTAI agent host process.", aliases: ["restart"], category: "settings", behavior: "action" },
];

export const SLASH_COMMAND_INDEX: readonly SlashCommandMeta[] =
  Object.freeze(COMMANDS);

export function findSlashCommands(query = ""): readonly SlashCommandMeta[] {
  const normalized = query.trim().toLowerCase();
  if (!normalized) {
    return SLASH_COMMAND_INDEX;
  }
  return SLASH_COMMAND_INDEX.filter((command) =>
    [
      command.name,
      command.label,
      command.description,
      ...(command.aliases ?? []),
      command.category,
    ].some((value) => value.toLowerCase().includes(normalized)),
  );
}

export function resolveSlashCommand(
  name: string,
): SlashCommandMeta | undefined {
  const normalized = name.trim().toLowerCase();
  return SLASH_COMMAND_INDEX.find(
    (command) =>
      command.name === normalized || command.aliases?.includes(normalized),
  );
}

/**
 * Format a compact help digest for the transcript meta line.
 * Optional filter prefers matching category tokens or command names.
 */
export function formatSlashHelpDigest(
  filter = "",
  commands: readonly SlashCommandMeta[] = SLASH_COMMAND_INDEX,
): string {
  const needle = filter.trim().toLowerCase();
  const list = needle
    ? commands.filter((command) =>
        [
          command.name,
          command.category,
          command.label,
          ...(command.aliases ?? []),
        ].some((value) => value.toLowerCase().includes(needle)),
      )
    : [...commands];
  if (list.length === 0) {
    return `No slash commands match “${filter.trim()}”. Try /help.`;
  }
  const lines = list.map(
    (command) => `${command.invocation} — ${command.description}`,
  );
  const head = needle
    ? `Slash commands matching “${filter.trim()}”:`
    : "Slash commands (type / for autocomplete):";
  // Cap so a meta message stays readable in the side panel.
  const body = lines.slice(0, 40);
  const more =
    lines.length > body.length
      ? `\n…and ${lines.length - body.length} more (narrow with /help <query>).`
      : "";
  return `${head}\n${body.join("\n")}${more}`;
}

/**
 * Parse a full composer line (or selection) as a slash command.
 * Only whole-line leading `/name ...` is recognized (first character `/`).
 */
export function tryRunSlashCommand(input: string): SlashOutcome {
  const trimmed = input.trim();
  if (!trimmed.startsWith("/")) {
    return { kind: "none" };
  }
  const [head, ...rest] = trimmed.slice(1).split(/\s+/);
  if (!head) {
    return { kind: "none" };
  }
  const command = resolveSlashCommand(head);
  if (!command) {
    return { kind: "none" };
  }
  const tail = rest.join(" ").trim();

  if (command.behavior === "prompt") {
    return {
      kind: "send-prompt",
      commandName: command.name,
      prompt: promptFor(command.name, tail),
    };
  }

  if (command.name === "review" && tail) {
    return {
      kind: "send-prompt",
      commandName: command.name,
      prompt: promptFor(command.name, tail),
    };
  }

  const toast = toastFor(command.name, tail);
  return {
    kind: "handled",
    action: command.name as SlashHostAction,
    tail,
    ...(toast ? { toast } : {}),
  };
}

function toastFor(name: string, tail: string): string | undefined {
  switch (name) {
    case "new":
      return "Started a new chat";
    case "sessions":
      return "Opened chat sessions";
    case "rename":
      return tail ? "Rename requested" : "Usage: /rename <new title>";
    case "retry":
      return "Retrying the last request";
    case "stop":
      return "Cancellation requested";
    case "compact":
      return "Compaction requested";
    case "status":
      return "Opened run details";
    case "plan":
      return tail === "off" || tail === "exit" ? "Plan mode off" : "Plan mode toggled";
    case "review":
      return "Opened change review";
    case "tasks":
      return "Opened Operations work";
    case "inbox":
      return "Opened Operations inbox";
    case "automations":
      return "Opened Operations scheduled";
    case "new-task":
      return "Opened new task composer";
    case "new-automation":
      return "Opened new automation composer";
    case "models":
    case "permissions":
    case "mcp":
    case "skills":
    case "settings":
      return "Opened Settings";
    case "logs":
      return "Opening ALTAI logs";
    case "diagnostics":
      return "Running diagnostics";
    case "restart-host":
      return "Restarting agent host";
    case "version":
      return "Showing version compatibility";
    case "copy":
      return undefined;
    case "connect":
      return "Opening provider connection";
    case "disconnect":
      return "Opening credential removal";
    case "walkthrough":
      return "Opening Getting Started walkthrough";
    case "extension-settings":
      return "Opening ALTAI extension settings";
    case "copy-diag":
      return "Copying diagnostics report";
    case "attach-problems":
      return "Attaching file problems";
    case "pick-root":
      return "Picking project root";
    case "attach-diff":
      return undefined;
    case "attach-terminal":
      return undefined;
    case "attach-file":
      return undefined;
    case "attach-selection":
      return undefined;
    case "help":
      return undefined;
    default:
      return undefined;
  }
}

function promptFor(name: string, tail: string): string {
  return promptForSlashCommand(name, tail);
}
