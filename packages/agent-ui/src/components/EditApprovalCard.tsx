import {
  CancelCircleIcon,
  CheckmarkCircle02Icon,
  File01Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useState } from "react";
import { cn } from "../lib/cn.js";

export type EditApprovalDiff = {
  file: string;
  diff: string;
  truncated: boolean;
};

export type EditApprovalCardProps = {
  diff: EditApprovalDiff;
  /** Host sends approve/deny (Desktop historically used chatStore.sendMessage). */
  onRespond: (choice: "approve" | "deny") => void;
};

const BTN =
  "inline-flex h-7 items-center justify-center gap-1 rounded-md px-2 text-[11px] font-medium transition-colors outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/30";

/**
 * Inline diff-review card for edit-gate approval (permission mode "ask").
 * Host owns the response transport via `onRespond`.
 */
export function EditApprovalCard({ diff, onRespond }: EditApprovalCardProps) {
  const [sent, setSent] = useState<"approve" | "deny" | null>(null);

  const reply = (choice: "approve" | "deny") => {
    if (sent) return;
    setSent(choice);
    onRespond(choice);
  };

  const lines = parseDiffLines(diff.diff);

  return (
    <div
      role="group"
      aria-label={`Edit approval for ${diff.file}`}
      className="altai-ai-edit-approval flex shrink-0 flex-col gap-2 rounded-md border border-border-subtle bg-warning/[0.06] px-3 py-2.5"
    >
      <div className="flex items-center gap-2">
        <span className="size-1.5 shrink-0 rounded-full bg-warning" />
        <HugeiconsIcon
          icon={File01Icon}
          size={13}
          strokeWidth={1.75}
          className="shrink-0 text-muted-foreground"
        />
        <span className="shrink-0 text-[11px] font-medium text-foreground">
          Edit approval
        </span>
        <span className="min-w-0 flex-1 truncate font-mono text-[11px] text-muted-foreground">
          {diff.file}
        </span>
        {diff.truncated ? (
          <span className="shrink-0 rounded bg-warning/15 px-1.5 py-0.5 text-[9.5px] font-medium text-warning">
            truncated
          </span>
        ) : null}
      </div>

      <div className="max-h-56 min-h-0 overflow-auto rounded-md border border-border/40 bg-background/60">
        <pre className="m-0 px-2 py-1.5 font-mono text-[10.5px] leading-relaxed">
          {lines.length > 0 ? (
            lines.map((ln, i) => (
              <span
                // eslint-disable-next-line react/no-array-index-key
                key={i}
                className={cn(
                  "block whitespace-pre",
                  ln.kind === "add" && "bg-success/10 text-success",
                  ln.kind === "del" && "bg-destructive/10 text-destructive",
                  ln.kind === "hunk" && "text-info",
                  ln.kind === "meta" && "text-muted-foreground/70",
                )}
              >
                <span className="select-none opacity-60">{ln.gutter}</span>
                {ln.text}
              </span>
            ))
          ) : (
            <span className="whitespace-pre-wrap text-muted-foreground">
              {diff.diff || " "}
            </span>
          )}
        </pre>
      </div>

      <div className="flex items-center justify-end gap-1.5">
        {sent ? (
          <span className="text-[10.5px] text-muted-foreground">
            {sent === "approve" ? "Approved — sending…" : "Denied — sending…"}
          </span>
        ) : (
          <>
            <button
              type="button"
              onClick={() => reply("deny")}
              className={cn(
                BTN,
                "px-2 text-muted-foreground hover:bg-muted hover:text-foreground",
              )}
            >
              <HugeiconsIcon
                icon={CancelCircleIcon}
                size={12}
                strokeWidth={1.75}
              />
              Deny
            </button>
            <button
              type="button"
              onClick={() => reply("approve")}
              className={cn(
                BTN,
                "bg-primary px-2.5 text-primary-foreground hover:bg-primary/85",
              )}
            >
              <HugeiconsIcon
                icon={CheckmarkCircle02Icon}
                size={12}
                strokeWidth={1.75}
              />
              Approve
            </button>
          </>
        )}
      </div>
      <span aria-live="polite" className="sr-only">
        File edit approval requested for {diff.file}. Approve or deny.
      </span>
    </div>
  );
}

type DiffLine = {
  kind: "add" | "del" | "hunk" | "meta" | "ctx";
  gutter: string;
  text: string;
};

/** Parse a unified diff into per-line render hints. */
export function parseDiffLines(diff: string): DiffLine[] {
  if (!diff) return [];
  const out: DiffLine[] = [];
  let sawAny = false;
  for (const raw of diff.split("\n")) {
    if (raw.startsWith("+++") || raw.startsWith("---")) {
      out.push({ kind: "meta", gutter: raw.slice(0, 1), text: raw.slice(1) });
      sawAny = true;
      continue;
    }
    if (raw.startsWith("@@")) {
      out.push({ kind: "hunk", gutter: "@", text: raw.slice(1) });
      sawAny = true;
      continue;
    }
    if (raw.startsWith("+")) {
      out.push({ kind: "add", gutter: "+", text: raw.slice(1) });
      sawAny = true;
      continue;
    }
    if (raw.startsWith("-")) {
      out.push({ kind: "del", gutter: "-", text: raw.slice(1) });
      sawAny = true;
      continue;
    }
    if (raw.startsWith(" ")) {
      out.push({ kind: "ctx", gutter: " ", text: raw.slice(1) });
      sawAny = true;
      continue;
    }
    out.push({ kind: "ctx", gutter: " ", text: raw });
  }
  return sawAny ? out : [];
}
