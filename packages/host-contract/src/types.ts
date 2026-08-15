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
  /** Prefer when listing/selecting models spans multiple providers. */
  defaultProviderId?: string;
  /** Optional secondary model for host-defined fallthrough. */
  fallbackModelId?: string;
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

/** Canonical durable Work lifecycle. TaskRunInfo remains a legacy run alias. */
export type WorkState =
  | "backlog"
  | "ready"
  | "in_progress"
  | "in_review"
  | "done"
  | "cancelled";

export type WorkListFilter = "my_active" | "review" | "backlog" | "done";

/** Durable peer PM objects in the existing user-scoped work.db. */
export type WorkItemKind = "task" | "ticket" | "campaign";

export type WorkItem = {
  id: string;
  projectId: string;
  title: string;
  description: string;
  acceptanceCriteria: string;
  kind: WorkItemKind;
  parentWorkId?: string | null;
  state: WorkState;
  assigneeRef?: string | null;
  blocker?: string | null;
  revision: number;
  createdAtMs: number;
  updatedAtMs: number;
};

export type WorkCreateInput = {
  title: string;
  description?: string;
  acceptanceCriteria?: string;
  assigneeRef?: string;
  kind?: WorkItemKind;
  parentWorkId?: string;
};

export type WorkTransitionInput = {
  workId: string;
  expectedRevision: number;
  nextState: WorkState;
};

export type WorkRevisionInput = {
  workId: string;
  expectedRevision: number;
};

export type WorkAttemptPhase =
  | "queued"
  | "running"
  | "waiting"
  | "succeeded"
  | "failed"
  | "cancelled";

/** Durable execution identity for one canonical Work attempt. */
export type WorkAttempt = {
  id: string;
  workId: string;
  number: number;
  role: string;
  phase: WorkAttemptPhase;
  chatId?: string | null;
  sessionId?: string | null;
  runId?: string | null;
  inputJson?: string | null;
  resultJson?: string | null;
  createdAtMs: number;
  updatedAtMs: number;
};

/**
 * One immutable transition record from the Work store's event log. Appended
 * in the same transaction as the mutation it describes; `kind` is the
 * transition (`created`, `state_changed`, `attempt_started`,
 * `attempt_run_bound`, `attempt_finished`, `accepted`, `returned`) and
 * `payloadJson` carries the transition's typed detail.
 */
export type WorkEvent = {
  id: number;
  workId: string;
  kind: string;
  payloadJson: string;
  createdAtMs: number;
};

/**
 * A transition-log event joined with its Work's title — the row an audit
 * feed renders. Same facts as `WorkEvent`, plus the Work it happened to,
 * so every recorded decision and stop names its owner.
 */
export type AuditEvent = {
  id: number;
  workId: string;
  workTitle: string;
  kind: string;
  payloadJson: string;
  createdAtMs: number;
};

/**
 * Token usage attributed to one attempt through its chat binding.
 * `usage` is absent when the attempt never bound a chat — nothing to
 * attribute is a different fact from zero usage.
 */
export type WorkUsageTotals = {
  promptTokens: number;
  completionTokens: number;
  totalTokens: number;
  cacheReadTokens: number;
  cacheCreationTokens: number;
  eventCount: number;
};

/**
 * An attempt joined with its Work and with the token usage its chat
 * recorded in the agent event journal — the row a usage ledger renders.
 */
export type WorkUsage = {
  attemptId: string;
  workId: string;
  workTitle: string;
  number: number;
  phase: WorkAttemptPhase;
  chatId?: string | null;
  runId?: string | null;
  updatedAtMs: number;
  usage?: WorkUsageTotals | null;
};

/**
 * An attempt joined with its Work — the row a Runs hub renders. The
 * attempt's `phase` and the Work's `workState` are distinct axes; a
 * running attempt on in-review Work is not the same fact as either label.
 */
export type WorkRun = {
  id: string;
  workId: string;
  workTitle: string;
  workState: WorkState;
  number: number;
  role: string;
  phase: WorkAttemptPhase;
  chatId?: string | null;
  sessionId?: string | null;
  runId?: string | null;
  createdAtMs: number;
  updatedAtMs: number;
};

/**
 * One registered agent in the embedded registry. `status` is the agent
 * lifecycle (`active`, `paused`, `terminated` — terminated is final);
 * `reportsTo` is the org-chart reporting line, which the host keeps
 * acyclic.
 */
export type AgentRecord = {
  id: string;
  name: string;
  status: "active" | "paused" | "terminated";
  reportsTo?: string | null;
  createdAtMs: number;
  updatedAtMs: number;
};

export type AgentStatusInput = "active" | "paused" | "terminated";

export type WorkStartRunResult = {
  work: WorkItem;
  attempt: WorkAttempt;
};

export type WorkReviewInput = WorkRevisionInput & {
  accept: boolean;
  guidance?: string;
};

export type WorkInboxKind =
  | "review_required"
  | "approval"
  | "question"
  | "failed_attempt"
  | "blocked";

/** Source-backed Work attention condition; resolving the source removes it. */
export type WorkInboxItem = {
  id: string;
  workId: string;
  kind: WorkInboxKind;
  title: string;
  why: string;
  createdAtMs: number;
  attemptId?: string | null;
  chatId?: string | null;
  runId?: string | null;
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

/**
 * Host `type: "usage"` payload fields (snake_case from stdio journal;
 * adapters may also accept camelCase).
 */
export type UsageEventPayload = {
  type?: "usage";
  prompt_tokens?: number;
  completion_tokens?: number;
  total_tokens?: number;
  cache_read_tokens?: number;
  cache_creation_tokens?: number;
};
