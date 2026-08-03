import {
  CodeIcon,
  File01Icon,
  HashtagIcon,
  TerminalIcon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { memo } from "react";

/** Host-neutral chip describing stripped user-context blocks in a prompt. */
export type ContextChip =
  | { kind: "selection"; source: "terminal" | "editor"; lines: number }
  | { kind: "file"; name: string; lines: number }
  | { kind: "terminal"; name: string; lines: number }
  | { kind: "diff"; name: string; lines: number }
  | { kind: "folder"; name: string; lines: number }
  | { kind: "snippet"; name: string };

export type ContextChipsProps = {
  chips: ContextChip[];
};

/**
 * Compact transcript chips for editor/terminal/file/diff context attached to
 * a user message. Parsing stays in the host; this only renders typed chips.
 */
export const ContextChips = memo(function ContextChips({
  chips,
}: ContextChipsProps) {
  if (chips.length === 0) return null;
  return (
    <div className="mb-1 flex flex-wrap gap-1">
      {chips.map((c, i) => (
        <span
          key={`${c.kind}:${"name" in c ? c.name : c.source}:${i}`}
          className="inline-flex items-center gap-1 rounded-md border border-border/50 bg-card/60 px-1.5 py-0.5 text-[10.5px] text-muted-foreground"
        >
          {chipIcon(c)}
          <span className="font-medium text-foreground">{chipLabel(c)}</span>
          {"lines" in c && c.lines > 0 ? (
            <span className="opacity-70">· {c.lines}L</span>
          ) : null}
        </span>
      ))}
    </div>
  );
});

function chipIcon(c: ContextChip) {
  if (c.kind === "selection") {
    return (
      <HugeiconsIcon
        icon={c.source === "editor" ? CodeIcon : TerminalIcon}
        size={10}
        strokeWidth={1.75}
      />
    );
  }
  if (c.kind === "file") {
    return <HugeiconsIcon icon={File01Icon} size={10} strokeWidth={1.75} />;
  }
  if (c.kind === "terminal") {
    return <HugeiconsIcon icon={TerminalIcon} size={10} strokeWidth={1.75} />;
  }
  if (c.kind === "diff") {
    return <HugeiconsIcon icon={CodeIcon} size={10} strokeWidth={1.75} />;
  }
  if (c.kind === "folder") {
    return <HugeiconsIcon icon={File01Icon} size={10} strokeWidth={1.75} />;
  }
  return <HugeiconsIcon icon={HashtagIcon} size={10} strokeWidth={1.75} />;
}

function chipLabel(c: ContextChip): string {
  if (c.kind === "selection") {
    return c.source === "editor" ? "Editor selection" : "Terminal selection";
  }
  if (c.kind === "file") return c.name;
  if (c.kind === "terminal" || c.kind === "diff" || c.kind === "folder") {
    return c.name;
  }
  return `#${c.name}`;
}
