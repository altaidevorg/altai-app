import { PopoverContent } from "@/components/ui/popover";
import {
  ComposerSuggestionList,
  type ComposerSuggestionItem,
} from "@altai/agent-ui";
import { HugeiconsIcon } from "@hugeicons/react";
import type { SlashCommandMeta } from "../lib/slashCommands";
import type { Snippet } from "../lib/snippets";

export type PickerItem =
  | { kind: "snippet"; snippet: Snippet }
  | { kind: "command"; command: SlashCommandMeta };

type Props = {
  items: readonly PickerItem[];
  activeIndex: number;
  onPick: (item: PickerItem) => void;
  onHover: (index: number) => void;
  commandPrefix?: "#" | "/";
};

/**
 * Desktop adapter: Popover chrome + Desktop snippet/command types around the
 * shared suggestion list.
 */
export function SnippetPickerContent({
  items,
  activeIndex,
  onPick,
  onHover,
  commandPrefix = "#",
}: Props) {
  const sharedItems: ComposerSuggestionItem[] = items.map((it) => {
    if (it.kind === "command") {
      const c = it.command;
      return {
        kind: "command" as const,
        name: c.name,
        label: c.label,
        description: c.description,
        category: c.category,
        aliases: c.aliases,
        source: c.source,
        icon: (
          <HugeiconsIcon
            icon={c.icon}
            size={13}
            strokeWidth={1.75}
            className="text-muted-foreground"
          />
        ),
      };
    }
    const s = it.snippet;
    return {
      kind: "snippet" as const,
      id: s.id,
      handle: s.handle,
      name: s.name,
      description: s.description,
    };
  });

  return (
    <PopoverContent
      side="top"
      align="start"
      sideOffset={6}
      onOpenAutoFocus={(e) => e.preventDefault()}
      onCloseAutoFocus={(e) => e.preventDefault()}
      onMouseDown={(e) => e.preventDefault()}
      className="w-auto border-0 bg-transparent p-0 shadow-none"
    >
      <ComposerSuggestionList
        items={sharedItems}
        activeIndex={activeIndex}
        commandPrefix={commandPrefix}
        onHover={onHover}
        onPick={(shared) => {
          const original =
            shared.kind === "command"
              ? items.find(
                  (it) =>
                    it.kind === "command" && it.command.name === shared.name,
                )
              : items.find(
                  (it) =>
                    it.kind === "snippet" && it.snippet.id === shared.id,
                );
          if (original) onPick(original);
        }}
      />
    </PopoverContent>
  );
}
