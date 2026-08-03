import {
  Cancel01Icon,
  Edit02Icon,
  FileEditIcon,
  FilePlusIcon,
  FolderAddIcon,
  TerminalIcon,
  Tick02Icon,
  ToolsIcon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { memo, useId } from "react";
import { cn } from "../lib/cn.js";

/**
 * Minimal approval-request shape shared across hosts.
 * Compatible with AI SDK `ToolUIPart` approval-requested parts.
 */
export type ToolApprovalPart = {
  state: "approval-requested";
  approval: { id: string };
  input?: unknown;
};

export type AiToolApprovalProps = {
  part: ToolApprovalPart;
  toolName: string;
  onRespond: (approved: boolean) => void;
  /**
   * When true (default), the live region uses role="alert" so screen readers
   * interrupt streaming output. Hosts map this from accessibility prefs.
   */
  assertiveAnnounce?: boolean;
};

const TOOL_META: Record<string, { label: string; icon: typeof FilePlusIcon }> =
  {
    write_file: { label: "Write file", icon: FilePlusIcon },
    edit: { label: "Edit file", icon: FileEditIcon },
    multi_edit: { label: "Edit file (batch)", icon: Edit02Icon },
    create_directory: { label: "Create directory", icon: FolderAddIcon },
    bash_run: { label: "Run shell command", icon: TerminalIcon },
    bash_background: { label: "Spawn background process", icon: TerminalIcon },
  };

const BTN =
  "inline-flex h-7 items-center justify-center gap-1.5 rounded-md px-3 text-[11px] font-medium transition-colors outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/30";

function AiToolApprovalImpl({
  part,
  toolName,
  onRespond,
  assertiveAnnounce = true,
}: AiToolApprovalProps) {
  const meta = TOOL_META[toolName];
  const label = meta?.label ?? toolName;
  const Icon = meta?.icon ?? ToolsIcon;
  const input =
    part.input && typeof part.input === "object"
      ? (part.input as Record<string, unknown>)
      : {};
  const titleId = useId();

  return (
    <div
      role="group"
      aria-labelledby={titleId}
      className="altai-ai-approval min-w-0 max-w-full overflow-hidden rounded-md border border-border bg-card shadow-sm"
    >
      <div
        role={assertiveAnnounce ? "alert" : "status"}
        className="sr-only"
      >
        {label} requires approval
      </div>
      <div className="flex items-center gap-2 border-b border-border-subtle px-3 py-2">
        <span
          aria-hidden="true"
          className="size-1.5 shrink-0 rounded-full bg-warning animate-pulse"
        />
        <HugeiconsIcon
          icon={Icon}
          size={13}
          strokeWidth={1.75}
          className="shrink-0 text-muted-foreground"
          aria-hidden="true"
        />
        <span id={titleId} className="text-[12px] font-medium text-foreground">
          {label}
        </span>
        <span className="ml-auto text-[10px] text-muted-foreground">
          needs approval
        </span>
      </div>

      <div className="px-3 py-2.5">
        <PreviewBlock toolName={toolName} input={input} />
      </div>

      <div className="flex items-center justify-end gap-1.5 border-t border-border/60 px-3 py-2">
        <button
          type="button"
          onClick={() => onRespond(false)}
          className={cn(BTN, "hover:bg-muted hover:text-foreground")}
        >
          <HugeiconsIcon icon={Cancel01Icon} size={12} strokeWidth={2} />
          Deny
        </button>
        <button
          type="button"
          onClick={() => onRespond(true)}
          className={cn(
            BTN,
            "bg-primary text-primary-foreground hover:bg-primary/85",
          )}
        >
          <HugeiconsIcon icon={Tick02Icon} size={12} strokeWidth={2} />
          Approve
        </button>
      </div>
    </div>
  );
}

export const AiToolApproval = memo(AiToolApprovalImpl, (a, b) => {
  return (
    a.toolName === b.toolName &&
    a.part.approval.id === b.part.approval.id &&
    a.onRespond === b.onRespond &&
    a.assertiveAnnounce === b.assertiveAnnounce
  );
});

function PreviewBlock({
  toolName,
  input,
}: {
  toolName: string;
  input: Record<string, unknown>;
}) {
  if (toolName === "bash_run" || toolName === "bash_background") {
    const cwd = typeof input.cwd === "string" ? input.cwd : null;
    return (
      <div className="space-y-1.5">
        {cwd ? (
          <div className="break-all font-mono text-[10.5px] text-muted-foreground [overflow-wrap:anywhere]">
            {cwd}
          </div>
        ) : null}
        <pre className="max-h-40 max-w-full overflow-x-auto whitespace-pre-wrap break-words rounded-md bg-muted/60 p-2 font-mono text-[11px] leading-relaxed [overflow-wrap:anywhere]">
          {String(input.command ?? "")}
        </pre>
      </div>
    );
  }
  if (toolName === "write_file") {
    const content = typeof input.content === "string" ? input.content : "";
    const lines = content ? content.split("\n").length : 0;
    return (
      <div className="space-y-0.5 font-mono text-[11px]">
        <div className="break-all text-muted-foreground [overflow-wrap:anywhere]">
          {String(input.path ?? "")}
        </div>
        <div className="text-[10.5px] text-muted-foreground/80">
          {lines} line{lines === 1 ? "" : "s"} · review in the diff tab
        </div>
      </div>
    );
  }
  if (toolName === "edit") {
    const oldStr = typeof input.old_string === "string" ? input.old_string : "";
    const newStr = typeof input.new_string === "string" ? input.new_string : "";
    const removed = oldStr ? oldStr.split("\n").length : 0;
    const added = newStr ? newStr.split("\n").length : 0;
    return (
      <div className="space-y-0.5 font-mono text-[11px]">
        <div className="break-all text-muted-foreground [overflow-wrap:anywhere]">
          {String(input.path ?? "")}
          {input.replace_all ? " · replace all" : ""}
        </div>
        <div className="text-[10.5px] text-muted-foreground/80">
          −{removed} / +{added} line{added === 1 && removed === 1 ? "" : "s"} ·
          review in the diff tab
        </div>
      </div>
    );
  }
  if (toolName === "multi_edit") {
    const edits = Array.isArray(input.edits)
      ? (input.edits as Array<{ old_string?: string; new_string?: string }>)
      : [];
    return (
      <div className="space-y-0.5 font-mono text-[11px]">
        <div className="break-all text-muted-foreground [overflow-wrap:anywhere]">
          {String(input.path ?? "")}
        </div>
        <div className="text-[10.5px] text-muted-foreground/80">
          {edits.length} edit{edits.length === 1 ? "" : "s"} · review in the
          diff tab
        </div>
      </div>
    );
  }
  if (toolName === "create_directory") {
    return (
      <div className="break-all font-mono text-[11px] text-muted-foreground [overflow-wrap:anywhere]">
        {String(input.path ?? "")}
      </div>
    );
  }
  return (
    <pre className="max-w-full overflow-x-auto whitespace-pre-wrap break-words rounded-md bg-muted/60 p-2 font-mono text-[11px] leading-relaxed [overflow-wrap:anywhere]">
      {JSON.stringify(input, null, 2)}
    </pre>
  );
}
