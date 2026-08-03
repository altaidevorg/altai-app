import type { ReactNode } from "react";
import { cn } from "../lib/cn.js";

export type ComposerSuggestionCommand = {
  kind: "command";
  name: string;
  label: string;
  description: string;
  category: string;
  aliases?: readonly string[];
  source?: string;
  icon: ReactNode;
};

export type ComposerSuggestionSnippet = {
  kind: "snippet";
  id: string;
  handle: string;
  name: string;
  description?: string;
};

export type ComposerSuggestionItem =
  | ComposerSuggestionCommand
  | ComposerSuggestionSnippet;

export type ComposerSuggestionListProps = {
  items: readonly ComposerSuggestionItem[];
  activeIndex: number;
  onPick: (item: ComposerSuggestionItem) => void;
  onHover: (index: number) => void;
  commandPrefix?: "#" | "/";
};

/**
 * Slash-command / snippet suggestion list for the composer.
 * Hosts own the popover/portal chrome; this renders only the list body.
 */
export function ComposerSuggestionList({
  items,
  activeIndex,
  onPick,
  onHover,
  commandPrefix = "#",
}: ComposerSuggestionListProps) {
  const commands = items.filter(
    (it): it is ComposerSuggestionCommand => it.kind === "command",
  );
  const snippets = items.filter(
    (it): it is ComposerSuggestionSnippet => it.kind === "snippet",
  );
  let cursor = -1;

  return (
    <div className="w-72 overflow-hidden rounded-lg border border-border/80 bg-popover p-0 text-popover-foreground shadow-xl">
      {items.length === 0 ? (
        <div className="px-3 py-2.5 text-[11px] text-muted-foreground">
          {commandPrefix === "/"
            ? "No slash commands match."
            : "No matches. Add snippets in Settings → Agents."}
        </div>
      ) : (
        <div className="max-h-64 overflow-y-auto py-1">
          {commands.length > 0 ? (
            <>
              <SectionHeader
                label={commandPrefix === "/" ? "Slash commands" : "Commands"}
              />
              <ul>
                {commands.map((c) => {
                  cursor += 1;
                  const i = cursor;
                  return (
                    <li key={`cmd-${c.name}`}>
                      <button
                        type="button"
                        onMouseEnter={() => onHover(i)}
                        onClick={() => onPick(c)}
                        className={cn(
                          "mx-1 my-0.5 flex w-[calc(100%-0.5rem)] items-center gap-2 rounded-md px-2 py-1.5 text-left text-[12px] text-popover-foreground",
                          i === activeIndex
                            ? "bg-foreground/[0.065]"
                            : "hover:bg-foreground/[0.055]",
                        )}
                      >
                        {c.icon}
                        <span className="flex min-w-0 flex-1 flex-col">
                          <span className="flex items-center gap-1.5">
                            <span className="font-mono text-muted-foreground">
                              {commandPrefix}
                              {c.name}
                            </span>
                            <span className="font-medium">{c.label}</span>
                            <span className="rounded bg-foreground/[0.06] px-1 py-px text-[8.5px] font-medium uppercase tracking-wide text-muted-foreground">
                              {c.category}
                            </span>
                          </span>
                          <span className="line-clamp-1 text-[10.5px] text-muted-foreground">
                            {c.description}
                            {c.aliases?.length
                              ? // Slash-command aliases are always `/name`, even when the
                                // picker was opened from a `#` trigger mixed list.
                                ` · aliases: ${c.aliases.map((alias) => `/${alias}`).join(", ")}`
                              : ""}
                            {c.source === "workspace"
                              ? " · workspace workflow"
                              : ""}
                          </span>
                        </span>
                      </button>
                    </li>
                  );
                })}
              </ul>
            </>
          ) : null}
          {snippets.length > 0 ? (
            <>
              <SectionHeader label="Snippets" />
              <ul>
                {snippets.map((s) => {
                  cursor += 1;
                  const i = cursor;
                  return (
                    <li key={`sn-${s.id}`}>
                      <button
                        type="button"
                        onMouseEnter={() => onHover(i)}
                        onClick={() => onPick(s)}
                        className={cn(
                          "mx-1 my-0.5 flex w-[calc(100%-0.5rem)] flex-col items-start gap-0.5 rounded-md px-2 py-1.5 text-left text-[12px] text-popover-foreground",
                          i === activeIndex
                            ? "bg-foreground/[0.065]"
                            : "hover:bg-foreground/[0.055]",
                        )}
                      >
                        <span className="flex w-full items-center gap-1.5">
                          <span className="font-mono text-muted-foreground">
                            #{s.handle}
                          </span>
                          <span className="font-medium">{s.name}</span>
                        </span>
                        {s.description ? (
                          <span className="line-clamp-1 text-[10.5px] text-muted-foreground">
                            {s.description}
                          </span>
                        ) : null}
                      </button>
                    </li>
                  );
                })}
              </ul>
            </>
          ) : null}
        </div>
      )}
    </div>
  );
}

function SectionHeader({ label }: { label: string }) {
  return (
    <div className="px-2 pt-1.5 pb-1 text-[10px] font-medium uppercase tracking-wide text-muted-foreground/70">
      {label}
    </div>
  );
}
