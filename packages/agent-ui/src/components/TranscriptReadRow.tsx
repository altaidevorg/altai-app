import { File01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { cn } from "../lib/cn.js";
import { readPathFromToolPart, type ToolLikePart } from "../lib/transcriptToolGroups.js";

export type TranscriptReadRowProps = {
  part: ToolLikePart;
  className?: string;
};

/**
 * Compact single read_file row (not collapsed into a multi-file group).
 * Wave 4 / A6.6.
 */
export function TranscriptReadRow({ part, className }: TranscriptReadRowProps) {
  const path = readPathFromToolPart(part);
  const isError = (part.state ?? "") === "output-error";
  return (
    <div
      className={cn(
        "flex items-center gap-2 rounded-md px-2 py-1.5 text-[12px]",
        className,
      )}
    >
      <span
        className={cn(
          "size-1.5 shrink-0 rounded-full",
          isError
            ? "bg-destructive"
            : "border border-muted-foreground/40 bg-transparent",
        )}
      />
      <HugeiconsIcon
        icon={File01Icon}
        size={13}
        strokeWidth={1.75}
        className="shrink-0 text-muted-foreground"
      />
      <span className="shrink-0 font-medium text-foreground">Read</span>
      <span className="min-w-0 flex-1 truncate font-mono text-[11px] text-muted-foreground">
        {path ?? ""}
      </span>
    </div>
  );
}
