/**
 * Product-neutral DTOs for host ports.
 * Adapted from existing Desktop chat-panel behavior; no Tauri/VS Code types.
 */

export const HOST_CONTRACT_VERSION = 1 as const;

export const PERMISSION_MODES = [
  "ask",
  "auto-edit",
  "plan",
  "bypass",
] as const;

export type PermissionMode = (typeof PERMISSION_MODES)[number];

export type TextRange = {
  startLine: number;
  startCharacter: number;
  endLine: number;
  endCharacter: number;
};

export type InitializeInput = {
  protocolMin: number;
  protocolMax: number;
  clientName: string;
  clientVersion: string;
  workspaceRoots?: string[];
};

export type RunRef = {
  chatId: string;
  runId: string;
};

export type StartRunInput = {
  chatId?: string;
  prompt: string;
  modelId?: string;
  permissionMode?: PermissionMode;
  attachments?: RunAttachment[];
  queue?: boolean;
};

export type RunAttachment = {
  uri: string;
  name?: string;
  mimeType?: string;
};

export type SteerRunInput = {
  chatId: string;
  runId: string;
  prompt: string;
};

export type CancelRunInput = {
  chatId: string;
  runId: string;
};

export type RetryRunInput = {
  chatId: string;
  runId?: string;
  editUserMessage?: string;
};

export type ApprovalResponse = {
  chatId: string;
  runId: string;
  approvalId: string;
  decision: "approve" | "deny";
};

export type ClarificationResponse = {
  chatId: string;
  ticketId: string;
  action: "reply" | "dismiss";
  text?: string;
};

export type CompactContextInput = {
  chatId: string;
};

export type ReplayRunInput = {
  chatId: string;
  runId: string;
  afterSeq?: number;
  limit?: number;
};

export type ReplayCursor = {
  chatId: string;
  runId: string;
  seq: number;
};

export type ReplayPage = {
  events: AgentEvent[];
  cursor: ReplayCursor | null;
  exhausted: boolean;
};

export type WorkspaceInfo = {
  roots: string[];
  trusted: boolean;
  currentDir?: string;
};

export type FileContext = {
  uri: string;
  path: string;
  languageId?: string;
};

export type SelectionContext = {
  uri: string;
  path: string;
  range: TextRange;
  text: string;
};

export type FileMatch = {
  uri: string;
  path: string;
  score?: number;
};

export type FileContent = {
  uri: string;
  path: string;
  text: string;
  truncated: boolean;
};

export type DiffInput = {
  title?: string;
  originalUri?: string;
  modifiedUri?: string;
  originalText?: string;
  modifiedText?: string;
  path?: string;
};

export type GitDiffContext = {
  branch?: string;
  files: Array<{ path: string; status: string }>;
  patch?: string;
};

export type TerminalContext = {
  cwd?: string;
  selectedText?: string;
  lastCommand?: string;
};

export type SessionInfo = {
  id: string;
  title: string;
  updatedAt: string;
  archived?: boolean;
};

export type SessionMessage = {
  id: string;
  role: "user" | "assistant" | "system" | "tool";
  content: string;
  createdAt: string;
};

export type ModelInfo = {
  id: string;
  label: string;
  providerId: string;
};

export type ProviderStatus = {
  providerId: string;
  connected: boolean;
  label?: string;
  error?: string;
};

export type ProviderConnectionInput = {
  providerId: string;
  /** Host-owned secret material; never echoed back to UI. */
  secretRef?: string;
  baseUrl?: string;
};

export type AltaiSettings = {
  permissionMode: PermissionMode;
  bypassEnabled: boolean;
  defaultModelId?: string;
  compactionEnabled?: boolean;
  /** Opaque host-owned settings bag for host-specific keys. */
  extensions?: Record<string, unknown>;
};

export type SettingsPatch = Partial<AltaiSettings>;

export type CheckpointInfo = {
  id: string;
  chatId: string;
  createdAt: string;
  label?: string;
};

/** Planned file mutation awaiting Apply/Deny on the native review host. */
export type EditProposalKind =
  | "edit_file"
  | "create_file"
  | "create_directory"
  | "write_file"
  | "edit"
  | "multi_edit";

export type EditProposalInput = {
  path: string;
  kind?: EditProposalKind;
  /** Full content before the edit (empty for new files / directories). */
  originalContent?: string;
  /** Full content after the edit (empty for create_directory). */
  proposedContent?: string;
  chatId?: string;
  runId?: string;
};

export type EditProposalInfo = {
  id: string;
  path: string;
  kind: string;
  originalContent?: string;
  proposedContent?: string;
  chatId?: string;
  runId?: string;
  isNewFile?: boolean;
};

export type TaskRunInfo = {
  id: string;
  chatId?: string;
  title: string;
  status: "queued" | "running" | "succeeded" | "failed" | "cancelled";
  createdAt: string;
};

export type AutomationInfo = {
  id: string;
  /** Conversation that owns the scheduled agent turn. */
  chatId: string;
  title: string;
  /** Instruction injected into the owner conversation when the schedule fires. */
  prompt: string;
  schedule: { kind: "once"; at: string } | { kind: "every"; everyMs: number };
  enabled: boolean;
};

export type NotificationInfo = {
  id: string;
  title: string;
  body?: string;
  seen: boolean;
  createdAt: string;
  chatId?: string;
};

export type McpServerStatus = {
  id: string;
  name: string;
  enabled: boolean;
  connected: boolean;
  error?: string;
};

export type SkillInfo = {
  name: string;
  description?: string | null;
  enabled?: boolean;
};

export type AgentEventType =
  | "message"
  | "reasoning"
  | "tool"
  | "usage"
  | "diff"
  | "approval"
  | "clarification"
  | "subagent"
  | "lifecycle"
  | "notification"
  | "warning";

export type AgentEvent = {
  type: AgentEventType;
  chatId: string;
  runId: string;
  seq: number;
  payload: unknown;
};
