/**
 * Host ports consumed by shared UI. Adapters (Tauri / VS Code) implement these.
 * This package must never import `@tauri-apps/*` or `vscode`.
 */

import type {
  AgentEvent,
  AltaiSettings,
  ApprovalResponse,
  AutomationInfo,
  CancelRunInput,
  CheckpointInfo,
  ClarificationResponse,
  CompactContextInput,
  DiffInput,
  FileContent,
  FileContext,
  FileMatch,
  GitDiffContext,
  InitializeInput,
  McpServerStatus,
  ModelInfo,
  NotificationInfo,
  PermissionMode,
  ProviderConnectionInput,
  ProviderStatus,
  ReplayPage,
  ReplayRunInput,
  RetryRunInput,
  RunRef,
  SelectionContext,
  SessionInfo,
  SessionMessage,
  SettingsPatch,
  SkillInfo,
  StartRunInput,
  SteerRunInput,
  TaskRunInfo,
  TerminalContext,
  TextRange,
  WorkspaceInfo,
} from "./types.js";
import type { Capabilities } from "./capabilities.js";

export interface AgentRuntimePort {
  initialize(input: InitializeInput): Promise<Capabilities>;
  startRun(input: StartRunInput): Promise<RunRef>;
  steerRun(input: SteerRunInput): Promise<void>;
  cancelRun(input: CancelRunInput): Promise<void>;
  retryRun(input: RetryRunInput): Promise<RunRef>;
  respondToApproval(input: ApprovalResponse): Promise<void>;
  respondToClarification(input: ClarificationResponse): Promise<void>;
  compactContext(input: CompactContextInput): Promise<void>;
  replayRun(input: ReplayRunInput): Promise<ReplayPage>;
  shutdown(): Promise<void>;
}

export interface SessionPort {
  listSessions(): Promise<SessionInfo[]>;
  getSession(sessionId: string): Promise<SessionInfo | null>;
  createSession(title?: string): Promise<SessionInfo>;
  renameSession(sessionId: string, title: string): Promise<SessionInfo>;
  archiveSession(sessionId: string): Promise<void>;
  deleteSession(sessionId: string): Promise<void>;
  truncateSession(sessionId: string, afterMessageId: string): Promise<void>;
  listMessages(sessionId: string): Promise<SessionMessage[]>;
}

export interface WorkspacePort {
  getWorkspace(): Promise<WorkspaceInfo>;
  getActiveFile(): Promise<FileContext | null>;
  getSelection(): Promise<SelectionContext | null>;
  searchFiles(query: string): Promise<FileMatch[]>;
  readFile(uri: string): Promise<FileContent>;
  openFile(uri: string, range?: TextRange): Promise<void>;
  openDiff(input: DiffInput): Promise<void>;
  getGitDiff(): Promise<GitDiffContext | null>;
  getTerminalContext(): Promise<TerminalContext | null>;
}

export interface SettingsPort {
  getSettings(): Promise<AltaiSettings>;
  updateSettings(patch: SettingsPatch): Promise<AltaiSettings>;
  getProviderStatus(): Promise<ProviderStatus[]>;
  beginProviderConnection(input: ProviderConnectionInput): Promise<void>;
  clearProviderCredential(providerId: string): Promise<void>;
  listModels(): Promise<ModelInfo[]>;
  setPermissionMode(mode: PermissionMode): Promise<PermissionMode>;
}

export interface ReviewPort {
  listCheckpoints(chatId: string): Promise<CheckpointInfo[]>;
  restoreCheckpoint(checkpointId: string): Promise<void>;
  applyEditProposal(proposalId: string): Promise<void>;
  denyEditProposal(proposalId: string): Promise<void>;
}

export interface WorkPort {
  listTaskRuns(): Promise<TaskRunInfo[]>;
  createTaskRun(input: {
    title: string;
    prompt: string;
    permissionMode?: PermissionMode;
  }): Promise<TaskRunInfo>;
  cancelTaskRun(taskRunId: string): Promise<void>;
  retryTaskRun(taskRunId: string): Promise<TaskRunInfo>;
  removeTaskRun(taskRunId: string): Promise<void>;
  /** Automations always retain both their owning chat and run instruction. */
  listAutomations(): Promise<AutomationInfo[]>;
  createAutomation(input: Omit<AutomationInfo, "id">): Promise<AutomationInfo>;
  updateAutomation(
    id: string,
    patch: Partial<Omit<AutomationInfo, "id">>,
  ): Promise<AutomationInfo>;
  triggerAutomation(id: string): Promise<void>;
  pauseAutomation(id: string): Promise<void>;
  deleteAutomation(id: string): Promise<void>;
}

export interface InboxPort {
  listNotifications(): Promise<NotificationInfo[]>;
  markNotificationSeen(id: string): Promise<void>;
  resolveNotification(id: string): Promise<void>;
  dismissNotification(id: string): Promise<void>;
}

export interface McpSkillsPort {
  listMcpServers(): Promise<McpServerStatus[]>;
  configureMcpServer(id: string, config: unknown): Promise<McpServerStatus>;
  setMcpServerEnabled(id: string, enabled: boolean): Promise<void>;
  restartMcpServer(id: string): Promise<void>;
  listSkills(): Promise<SkillInfo[]>;
  installSkill(source: string): Promise<SkillInfo>;
  setSkillEnabled(name: string, enabled: boolean): Promise<void>;
}

export interface EventPort {
  subscribe(listener: (event: AgentEvent) => void): () => void;
}

/**
 * Aggregate host surface used by shared UI once adapters are wired.
 * Individual ports may be capability-gated; missing ports must not enable UI.
 */
export type HostPorts = {
  runtime: AgentRuntimePort;
  sessions: SessionPort;
  workspace: WorkspacePort;
  settings: SettingsPort;
  review: ReviewPort;
  work: WorkPort;
  inbox: InboxPort;
  mcpSkills: McpSkillsPort;
  events: EventPort;
};
