import {
  ArrowDown01Icon,
  Folder01Icon,
  GithubIcon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";

export type ChatProjectTargetProps = {
  name: string;
  path: string | null;
  kind: "local" | "github" | null;
  onChange: () => void;
};

/**
 * Compact project-target chip under the composer. Purely presentational;
 * the host supplies the selected project and change handler.
 */
export function ChatProjectTarget({
  name,
  path,
  kind,
  onChange,
}: ChatProjectTargetProps) {
  const label = path ? name : "Choose a project";
  const detail =
    kind === "github"
      ? "GitHub repository"
      : path
        ? "Local folder"
        : "Optional · Local folder or GitHub";

  return (
    <div className="flex min-w-0 shrink-0 px-3 pb-2 pt-1">
      <button
        type="button"
        onClick={onChange}
        className="group flex h-8 min-w-0 max-w-full items-center gap-2 rounded-lg border border-border/70 bg-muted/25 px-2.5 text-left text-muted-foreground transition-colors hover:border-border hover:bg-accent hover:text-foreground"
        aria-label={
          path ? `Change project, currently ${name}` : "Choose a project"
        }
      >
        <HugeiconsIcon
          icon={kind === "github" ? GithubIcon : Folder01Icon}
          size={13}
          strokeWidth={1.75}
          className="shrink-0"
        />
        <span className="min-w-0 truncate text-[10.5px] font-medium text-foreground">
          {label}
        </span>
        <span className="hidden shrink-0 text-[9.5px] text-muted-foreground @[28rem]:inline">
          {detail}
        </span>
        <HugeiconsIcon
          icon={ArrowDown01Icon}
          size={11}
          strokeWidth={2}
          className="shrink-0 text-muted-foreground/70"
        />
      </button>
    </div>
  );
}
