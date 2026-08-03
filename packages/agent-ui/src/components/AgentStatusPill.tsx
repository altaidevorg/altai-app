import {
  AlertCircleIcon,
  Loading03Icon,
  ShieldUserIcon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import type { ReactNode } from "react";
import { cn } from "../lib/cn.js";

/** Host-neutral slice of agent run status needed by the status pill. */
export type AgentStatusMeta = {
  status:
    | "idle"
    | "thinking"
    | "streaming"
    | "awaiting-approval"
    | "cancelling"
    | "error";
  step: string | null;
  approvalsPending: number;
  error: string | null;
  activeSubagentCount: number;
};

export type AgentStatusPillProps = {
  meta: AgentStatusMeta;
  /**
   * Map raw tool / step names to friendly labels (Desktop: `toolLabel`).
   * Unknown names should pass through unchanged.
   */
  formatStepLabel: (step: string) => string;
  /**
   * When true, treat the error string as a recoverable pause (not a hard
   * failure). Defaults to messages starting with "Run paused".
   */
  isRecoverableAttention?: (message: string) => boolean;
  /** When provided the pill is a button (e.g. opens the AI log). */
  onClick?: () => void;
  /** Show a "Thinking…" fallback before the first agent event lands. */
  busy?: boolean;
  /** Suppress error / recoverable-attention states (chat owns those alerts). */
  hideError?: boolean;
  /** Own the sr-only live region (only one mounted instance should). */
  announce?: boolean;
};

function defaultRecoverableAttention(message: string): boolean {
  return message.startsWith("Run paused");
}

function plural(n: number, noun: string): string {
  return n === 1 ? `${n} ${noun}` : `${n} ${noun}s`;
}

/**
 * Compact live status chip for the agent run (thinking / tool / approval /
 * error). Hosts supply `meta` and step formatting; no store coupling.
 */
export function AgentStatusPill({
  meta,
  formatStepLabel,
  isRecoverableAttention = defaultRecoverableAttention,
  onClick,
  busy = false,
  hideError = false,
  announce = true,
}: AgentStatusPillProps) {
  const subCount = meta.activeSubagentCount;
  const active =
    busy || meta.status !== "idle" || Boolean(meta.error) || subCount > 0;
  if (!active) return null;
  if (
    hideError &&
    (meta.status === "error" ||
      (meta.error != null && isRecoverableAttention(meta.error)))
  ) {
    return null;
  }

  const { tone, icon, label } = describe(
    meta,
    subCount,
    formatStepLabel,
    isRecoverableAttention,
  );
  const subLabel = subCount > 0 ? `${plural(subCount, "subagent")} running` : "";
  const subSuffix = subLabel ? `, ${subLabel}` : "";
  const className = cn(
    "altai-ai-status flex h-6 items-center gap-1.5 rounded-md border px-1.5 text-[11px] transition-colors",
    tone,
  );
  const inner = (
    <>
      {icon}
      <span className="max-w-[180px] truncate">{label}</span>
      {subCount > 0 ? (
        <span
          aria-hidden="true"
          className="ml-0.5 rounded bg-muted px-1 text-[10px] font-medium tabular-nums text-foreground/80"
          title={subLabel}
        >
          {subCount} sub
        </span>
      ) : null}
    </>
  );

  return (
    <>
      {announce ? (
        <span role="status" aria-live="polite" className="sr-only">
          Agent status: {label}
          {subSuffix}
        </span>
      ) : null}
      {onClick ? (
        <button
          type="button"
          onClick={onClick}
          className={cn(className, "hover:bg-muted/40")}
          aria-label={`Open AI log — ${label}`}
        >
          {inner}
        </button>
      ) : (
        <div className={className}>{inner}</div>
      )}
    </>
  );
}

function describe(
  meta: AgentStatusMeta,
  subCount: number,
  formatStepLabel: (step: string) => string,
  isRecoverableAttention: (message: string) => boolean,
): {
  tone: string;
  icon: ReactNode;
  label: string;
} {
  if (meta.status === "awaiting-approval") {
    return {
      tone:
        "border-warning/40 bg-warning/10 text-warning hover:bg-warning/15",
      icon: (
        <HugeiconsIcon icon={ShieldUserIcon} size={12} strokeWidth={1.75} />
      ),
      label:
        meta.approvalsPending > 1
          ? `${meta.approvalsPending} approvals needed`
          : "Approval needed",
    };
  }
  if (meta.error && isRecoverableAttention(meta.error)) {
    return {
      tone:
        "border-warning/40 bg-warning/10 text-warning hover:bg-warning/15",
      icon: (
        <HugeiconsIcon icon={AlertCircleIcon} size={12} strokeWidth={1.75} />
      ),
      label: "Needs attention",
    };
  }
  if (meta.status === "error") {
    return {
      tone:
        "border-destructive/40 bg-destructive/10 text-destructive hover:bg-destructive/15",
      icon: (
        <HugeiconsIcon icon={AlertCircleIcon} size={12} strokeWidth={1.75} />
      ),
      label: meta.error ?? "Error",
    };
  }
  const fallback =
    meta.status === "idle" && subCount > 0 ? "Subagents running" : "Thinking…";
  return {
    tone:
      "border-border/60 bg-card text-muted-foreground hover:text-foreground",
    icon: (
      <HugeiconsIcon
        icon={Loading03Icon}
        size={12}
        strokeWidth={2}
        role="status"
        aria-label="Loading"
        className="size-3 animate-spin"
      />
    ),
    label: meta.step ? formatStepLabel(meta.step) : fallback,
  };
}
