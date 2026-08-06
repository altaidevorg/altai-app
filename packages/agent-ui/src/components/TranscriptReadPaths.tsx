import { File01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { ChatPathLink } from "./ChatPathLink.js";
import { pathBasename } from "../lib/transcriptToolGroups.js";

export type TranscriptReadPathsProps = {
  paths: readonly string[];
  onOpen: (path: string) => void;
};

/**
 * Expanded body for a Read tool group: basename + full path links.
 * Wave 4 / A6.4.
 */
export function TranscriptReadPaths({
  paths,
  onOpen,
}: TranscriptReadPathsProps) {
  if (paths.length === 0) {
    return null;
  }
  return (
    <ul className="flex flex-col gap-0.5 px-2 py-1.5">
      {paths.map((path) => (
        <li
          key={path}
          className="flex items-center gap-1.5 font-mono text-[11px] text-muted-foreground"
        >
          <HugeiconsIcon
            icon={File01Icon}
            size={10}
            strokeWidth={1.75}
            className="shrink-0 opacity-60"
          />
          <ChatPathLink
            path={path}
            onOpen={onOpen}
            className="truncate text-foreground hover:text-foreground"
          >
            {pathBasename(path)}
          </ChatPathLink>
          <ChatPathLink
            path={path}
            onOpen={onOpen}
            className="truncate opacity-60 hover:opacity-100"
          >
            {path}
          </ChatPathLink>
        </li>
      ))}
    </ul>
  );
}
