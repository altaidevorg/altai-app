/**
 * Versioned capability document returned by AgentRuntimePort.initialize.
 * UI controls enable only when the matching capability is `available`.
 */

import { HOST_CONTRACT_VERSION } from "./types.js";

export type CapabilityAvailability = "available" | "deferred" | "unsupported";

export type CapabilityId =
  // Lifecycle / runtime
  | "runtime.initialize"
  | "runtime.startRun"
  | "runtime.steerRun"
  | "runtime.cancelRun"
  | "runtime.retryRun"
  | "runtime.queueRun"
  | "runtime.compactContext"
  | "runtime.replayRun"
  | "runtime.shutdown"
  | "runtime.events"
  // Interactive
  | "interactive.approval"
  | "interactive.clarification"
  | "interactive.permissionModes"
  | "interactive.permissionBypass"
  // Sessions
  | "sessions.list"
  | "sessions.get"
  | "sessions.create"
  | "sessions.rename"
  | "sessions.archive"
  | "sessions.delete"
  | "sessions.truncate"
  | "sessions.messages"
  // Models / settings
  | "models.list"
  | "models.select"
  | "settings.get"
  | "settings.update"
  | "settings.providerStatus"
  | "settings.providerConnect"
  | "settings.providerClear"
  // Workspace / review
  | "workspace.info"
  | "workspace.activeFile"
  | "workspace.selection"
  | "workspace.searchFiles"
  | "workspace.readFile"
  | "workspace.openFile"
  | "workspace.openDiff"
  | "workspace.gitDiff"
  | "workspace.terminalContext"
  | "review.checkpoints"
  | "review.restoreCheckpoint"
  | "review.editProposal"
  // Work / Inbox
  | "work.items"
  | "work.attemptRuns"
  | "work.inbox"
  | "work.taskRuns"
  | "work.automations"
  | "inbox.notifications"
  // MCP / skills
  | "mcp.list"
  | "mcp.configure"
  | "skills.list"
  | "skills.install"
  // Explicitly later / Desktop-IDE-only for v1 shared UI
  | "desktop.gitPanelMutations"
  | "desktop.orchestration"
  | "desktop.gardening"
  | "desktop.shellSessions"
  | "desktop.pdfExtract"
  | "desktop.githubDeviceFlow"
  | "desktop.studioWindow";

export type CapabilityEntry = {
  id: CapabilityId;
  availability: CapabilityAvailability;
  /** Short reason when deferred/unsupported. */
  note?: string;
};

export type Capabilities = {
  contractVersion: typeof HOST_CONTRACT_VERSION;
  protocolVersion: number;
  hostName: string;
  hostVersion: string;
  capabilities: CapabilityEntry[];
};

export type ChatPanelAction = {
  /** Stable action id used in tests and capability gating docs. */
  action: string;
  /** Human-readable chat-panel surface. */
  surface: string;
  capability: CapabilityId;
};

/**
 * Catalog of existing chat-panel actions mapped to capabilities.
 * Every action is either wired to an `available`/`deferred`/`unsupported`
 * capability so UI never ships an enabled placeholder.
 */
export const CHAT_PANEL_ACTIONS: readonly ChatPanelAction[] = [
  { action: "composer.send", surface: "composer", capability: "runtime.startRun" },
  { action: "composer.steer", surface: "composer", capability: "runtime.steerRun" },
  { action: "composer.queue", surface: "composer", capability: "runtime.queueRun" },
  { action: "composer.stop", surface: "composer", capability: "runtime.cancelRun" },
  { action: "composer.retry", surface: "composer", capability: "runtime.retryRun" },
  { action: "composer.compact", surface: "composer", capability: "runtime.compactContext" },
  { action: "composer.attachFile", surface: "composer", capability: "workspace.readFile" },
  { action: "composer.permissionMode", surface: "composer", capability: "interactive.permissionModes" },
  { action: "composer.modelSelect", surface: "composer", capability: "models.select" },

  { action: "chat.newSession", surface: "history", capability: "sessions.create" },
  { action: "chat.openHistory", surface: "history", capability: "sessions.list" },
  { action: "chat.switchSession", surface: "history", capability: "sessions.get" },
  { action: "chat.renameSession", surface: "history", capability: "sessions.rename" },
  { action: "chat.archiveSession", surface: "history", capability: "sessions.archive" },
  { action: "chat.deleteSession", surface: "history", capability: "sessions.delete" },
  { action: "chat.truncate", surface: "history", capability: "sessions.truncate" },
  { action: "chat.reloadTranscript", surface: "chat", capability: "sessions.messages" },
  { action: "chat.replay", surface: "chat", capability: "runtime.replayRun" },

  { action: "approval.approve", surface: "approval", capability: "interactive.approval" },
  { action: "approval.deny", surface: "approval", capability: "interactive.approval" },
  { action: "clarification.reply", surface: "clarification", capability: "interactive.clarification" },
  { action: "clarification.dismiss", surface: "clarification", capability: "interactive.clarification" },
  { action: "permission.bypassConfirm", surface: "permissions", capability: "interactive.permissionBypass" },

  { action: "review.openDiff", surface: "review", capability: "workspace.openDiff" },
  { action: "review.applyEdit", surface: "review", capability: "review.editProposal" },
  { action: "review.denyEdit", surface: "review", capability: "review.editProposal" },
  { action: "review.listCheckpoints", surface: "review", capability: "review.checkpoints" },
  { action: "review.restoreCheckpoint", surface: "review", capability: "review.restoreCheckpoint" },

  { action: "context.activeFile", surface: "context", capability: "workspace.activeFile" },
  { action: "context.selection", surface: "context", capability: "workspace.selection" },
  { action: "context.searchFiles", surface: "context", capability: "workspace.searchFiles" },
  { action: "context.openFile", surface: "context", capability: "workspace.openFile" },
  { action: "context.gitDiff", surface: "context", capability: "workspace.gitDiff" },
  { action: "context.terminal", surface: "context", capability: "workspace.terminalContext" },

  { action: "work.open", surface: "work", capability: "work.items" },
  { action: "work.create", surface: "work", capability: "work.items" },
  { action: "work.transition", surface: "work", capability: "work.items" },
  { action: "work.start", surface: "work", capability: "work.items" },
  { action: "work.startRun", surface: "work", capability: "work.attemptRuns" },
  { action: "work.openRun", surface: "work", capability: "work.attemptRuns" },
  { action: "work.readyForReview", surface: "work", capability: "work.items" },
  { action: "work.review", surface: "work", capability: "work.items" },
  { action: "work.createTaskRun", surface: "work", capability: "work.taskRuns" },
  { action: "work.cancelTaskRun", surface: "work", capability: "work.taskRuns" },
  { action: "work.retryTaskRun", surface: "work", capability: "work.taskRuns" },
  { action: "work.automations", surface: "work", capability: "work.automations" },

  { action: "inbox.open", surface: "inbox", capability: "work.inbox" },
  { action: "inbox.markSeen", surface: "inbox", capability: "inbox.notifications" },
  { action: "inbox.resolve", surface: "inbox", capability: "inbox.notifications" },
  { action: "inbox.dismiss", surface: "inbox", capability: "inbox.notifications" },

  { action: "settings.open", surface: "settings", capability: "settings.get" },
  { action: "settings.update", surface: "settings", capability: "settings.update" },
  { action: "settings.providerConnect", surface: "settings", capability: "settings.providerConnect" },
  { action: "settings.providerClear", surface: "settings", capability: "settings.providerClear" },
  { action: "settings.mcp", surface: "settings", capability: "mcp.list" },
  { action: "settings.skills", surface: "settings", capability: "skills.list" },

  // Represented but deferred for shared UI / VS Code v1
  { action: "desktop.gitCommitPush", surface: "git", capability: "desktop.gitPanelMutations" },
  { action: "desktop.orchestration", surface: "orchestration", capability: "desktop.orchestration" },
  { action: "desktop.gardening", surface: "gardening", capability: "desktop.gardening" },
  { action: "desktop.shellSessions", surface: "terminal", capability: "desktop.shellSessions" },
  { action: "desktop.pdfExtract", surface: "attachments", capability: "desktop.pdfExtract" },
  { action: "desktop.githubConnect", surface: "settings", capability: "desktop.githubDeviceFlow" },
  { action: "desktop.studioWindow", surface: "studio", capability: "desktop.studioWindow" },
] as const;

/** Baseline capability matrix for a host that has not negotiated yet. */
export const DEFAULT_CAPABILITY_MATRIX: readonly CapabilityEntry[] = [
  { id: "runtime.initialize", availability: "available" },
  { id: "runtime.startRun", availability: "available" },
  { id: "runtime.steerRun", availability: "available" },
  { id: "runtime.cancelRun", availability: "available" },
  { id: "runtime.retryRun", availability: "deferred", note: "Requires interactive parity (TASK-011)" },
  { id: "runtime.queueRun", availability: "available" },
  { id: "runtime.compactContext", availability: "available" },
  { id: "runtime.replayRun", availability: "available" },
  { id: "runtime.shutdown", availability: "available" },
  { id: "runtime.events", availability: "available" },

  { id: "interactive.approval", availability: "deferred", note: "Requires TASK-011" },
  { id: "interactive.clarification", availability: "deferred", note: "Requires TASK-011" },
  { id: "interactive.permissionModes", availability: "available" },
  { id: "interactive.permissionBypass", availability: "deferred", note: "Requires explicit confirmation flow" },

  { id: "sessions.list", availability: "available" },
  { id: "sessions.get", availability: "available" },
  { id: "sessions.create", availability: "available" },
  { id: "sessions.rename", availability: "deferred", note: "Requires session protocol expansion" },
  { id: "sessions.archive", availability: "deferred", note: "Requires session protocol expansion" },
  { id: "sessions.delete", availability: "deferred", note: "Requires session protocol expansion" },
  { id: "sessions.truncate", availability: "deferred", note: "Requires session protocol expansion" },
  { id: "sessions.messages", availability: "available", note: "Hosted via journal replay for TASK-005" },

  { id: "models.list", availability: "available" },
  { id: "models.select", availability: "deferred", note: "Requires settings persistence behind host" },
  { id: "settings.get", availability: "deferred", note: "Requires SettingsPort adapter" },
  { id: "settings.update", availability: "deferred", note: "Requires SettingsPort adapter" },
  { id: "settings.providerStatus", availability: "deferred", note: "Requires SettingsPort adapter" },
  { id: "settings.providerConnect", availability: "deferred", note: "Secrets stay behind Rust host" },
  { id: "settings.providerClear", availability: "deferred", note: "Secrets stay behind Rust host" },

  { id: "workspace.info", availability: "deferred", note: "Requires WorkspacePort adapter" },
  { id: "workspace.activeFile", availability: "deferred", note: "Requires TASK-010" },
  { id: "workspace.selection", availability: "deferred", note: "Requires TASK-010" },
  { id: "workspace.searchFiles", availability: "deferred", note: "Requires TASK-010" },
  { id: "workspace.readFile", availability: "deferred", note: "Requires TASK-010" },
  { id: "workspace.openFile", availability: "deferred", note: "Requires TASK-010" },
  { id: "workspace.openDiff", availability: "deferred", note: "Requires TASK-011" },
  { id: "workspace.gitDiff", availability: "deferred", note: "Requires TASK-010" },
  { id: "workspace.terminalContext", availability: "deferred", note: "Requires TASK-010" },
  { id: "review.checkpoints", availability: "deferred", note: "Requires TASK-011" },
  { id: "review.restoreCheckpoint", availability: "deferred", note: "Requires TASK-011" },
  {
    id: "review.editProposal",
    availability: "deferred",
    note:
      "Native hosts advertise available when review/proposals/apply+deny exist (Wave 1)",
  },

  { id: "work.items", availability: "deferred", note: "Requires canonical Work host adapter" },
  {
    id: "work.attemptRuns",
    availability: "deferred",
    note: "Requires bound Work start, attempt history, replay, and cancellation",
  },
  { id: "work.inbox", availability: "deferred", note: "Requires source-backed Work Inbox query" },
  { id: "work.taskRuns", availability: "deferred", note: "Legacy run surface during Work migration" },
  { id: "work.automations", availability: "deferred", note: "Requires TASK-012 / protocol Automations domain" },
  { id: "inbox.notifications", availability: "deferred", note: "Requires TASK-012 / protocol Work domain" },

  { id: "mcp.list", availability: "deferred", note: "Requires TASK-012" },
  { id: "mcp.configure", availability: "deferred", note: "Requires TASK-012" },
  { id: "skills.list", availability: "deferred", note: "Requires TASK-012" },
  { id: "skills.install", availability: "deferred", note: "Requires TASK-012" },

  { id: "desktop.gitPanelMutations", availability: "unsupported", note: "Desktop IDE surface; not shared UI v1" },
  { id: "desktop.orchestration", availability: "unsupported", note: "Desktop IDE surface; not shared UI v1" },
  { id: "desktop.gardening", availability: "unsupported", note: "Desktop IDE surface; not shared UI v1" },
  { id: "desktop.shellSessions", availability: "unsupported", note: "Desktop IDE surface; not shared UI v1" },
  { id: "desktop.pdfExtract", availability: "deferred", note: "Attachment helper; host-specific later" },
  { id: "desktop.githubDeviceFlow", availability: "deferred", note: "Settings section; host-specific later" },
  { id: "desktop.studioWindow", availability: "unsupported", note: "Desktop-only windowing" },
] as const;

export function createCapabilities(input: {
  protocolVersion: number;
  hostName: string;
  hostVersion: string;
  overrides?: Partial<Record<CapabilityId, CapabilityAvailability>>;
}): Capabilities {
  const capabilities = DEFAULT_CAPABILITY_MATRIX.map((entry) => {
    const availability = input.overrides?.[entry.id] ?? entry.availability;
    const next: CapabilityEntry = {
      id: entry.id,
      availability,
    };
    if (entry.note !== undefined) {
      next.note = entry.note;
    }
    return next;
  });

  return {
    contractVersion: HOST_CONTRACT_VERSION,
    protocolVersion: input.protocolVersion,
    hostName: input.hostName,
    hostVersion: input.hostVersion,
    capabilities,
  };
}

export function isCapabilityEnabled(
  capabilities: Capabilities,
  id: CapabilityId,
): boolean {
  const entry = capabilities.capabilities.find((item) => item.id === id);
  return entry?.availability === "available";
}

export function capabilityForAction(action: string): CapabilityId | null {
  const found = CHAT_PANEL_ACTIONS.find((item) => item.action === action);
  return found?.capability ?? null;
}
