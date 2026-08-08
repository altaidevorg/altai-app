/**
 * Pure helpers for Run details inspector summary chrome (A6.70).
 * Hosts supply run/blocked state + message role counts; no vscode/Tauri.
 */

export type RunDetailsStatus = "idle" | "running" | "blocked";

export type RunDetailsMessage = {
  role: string;
};

export type RunOverviewMetric = {
  label: string;
  value: string;
};

export type RunDetailsChromeInput = {
  hasActiveRun: boolean;
  blockedMessage: string | null;
  warningMessage?: string | null;
  chatId: string | null;
  messages: readonly RunDetailsMessage[];
  toolCount: number;
  editDiffCount: number;
  approvalsPending: number;
};

export function canShowRunDetailsChrome(input: {
  hasActiveRun: boolean;
  blockedMessage: string | null;
  warningMessage?: string | null;
}): boolean {
  return (
    input.hasActiveRun ||
    Boolean(input.blockedMessage?.trim()) ||
    Boolean(input.warningMessage?.trim())
  );
}

export function runDetailsStatus(input: {
  hasActiveRun: boolean;
  blockedMessage: string | null;
}): RunDetailsStatus {
  if (input.blockedMessage?.trim()) {
    return "blocked";
  }
  if (input.hasActiveRun) {
    return "running";
  }
  return "idle";
}

export function runDetailsSubtitle(input: {
  chatId: string | null;
  status: RunDetailsStatus;
}): string {
  if (input.status === "blocked") {
    return input.chatId
      ? `Blocked · chat ${input.chatId}`
      : "Blocked · last agent run";
  }
  if (input.status === "running") {
    return input.chatId
      ? `Running · chat ${input.chatId}`
      : "Agent run in progress";
  }
  return input.chatId ? `chat ${input.chatId}` : "No active run";
}

export function runDetailsTokenLabel(input: {
  hasActiveRun: boolean;
  status: RunDetailsStatus;
  /** Accumulated host usage for this run; omit when never observed. */
  totalTokens?: number | null;
}): string {
  if (
    typeof input.totalTokens === "number" &&
    Number.isFinite(input.totalTokens) &&
    input.totalTokens > 0
  ) {
    return `Tokens · ${formatTokenCount(input.totalTokens)}`;
  }
  if (input.status === "blocked") {
    return "Tokens · n/a";
  }
  if (input.hasActiveRun) {
    return "Tokens · live";
  }
  return "Tokens · —";
}

function formatTokenCount(n: number): string {
  if (n < 1000) {
    return String(Math.floor(n));
  }
  if (n < 10_000) {
    const tenths = Math.round(n / 100) / 10;
    return `${tenths}k`.replace(/\.0k$/, "k");
  }
  if (n < 1_000_000) {
    return `${Math.round(n / 1000)}k`;
  }
  const mills = Math.round(n / 100_000) / 10;
  return `${mills}M`.replace(/\.0M$/, "M");
}

export function runDetailsStepLabel(input: {
  step: string | null;
  blockedMessage: string | null;
  warningMessage?: string | null;
}): string | null {
  if (input.blockedMessage?.trim()) {
    return input.blockedMessage.trim();
  }
  if (input.warningMessage?.trim()) {
    return input.warningMessage.trim();
  }
  return input.step;
}

export function buildRunOverviewMetrics(input: {
  messages: readonly RunDetailsMessage[];
  toolCount: number;
  editDiffCount: number;
  approvalsPending: number;
}): RunOverviewMetric[] {
  const userCount = input.messages.filter((m) => m.role === "user").length;
  const assistantCount = input.messages.filter(
    (m) => m.role === "assistant",
  ).length;
  return [
    { label: "User turns", value: String(userCount) },
    { label: "Replies", value: String(assistantCount) },
    { label: "Tools", value: String(input.toolCount) },
    { label: "Edits", value: String(input.editDiffCount) },
    ...(input.approvalsPending > 0
      ? [
          {
            label: "Approvals",
            value: String(input.approvalsPending),
          },
        ]
      : []),
  ];
}

export function countToolMessages(
  messages: readonly { role: string }[],
): number {
  return messages.filter((m) => m.role === "tool").length;
}
