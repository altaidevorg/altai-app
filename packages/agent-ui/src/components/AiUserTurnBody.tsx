/**
 * Shared user-turn body chrome for Desktop AiChatView + flat VS Code bubbles
 * (A6.40). Hosts own layout wrappers (Message / article) and edit mode.
 */

import type { ReactNode } from "react";
import {
  CommandSnippet,
  type CommandSnippetMeta,
} from "./CommandSnippet.js";
import { ContextChips, type ContextChip } from "./ContextChips.js";
import { cn } from "../lib/cn.js";

export type AiUserTurnBodyProps = {
  commandName?: string | null;
  /** Optional host metadata for richer slash chip (icon + label). */
  commandMeta?: CommandSnippetMeta | null;
  chips?: readonly ContextChip[];
  text?: string | null;
  /**
   * When set, replaces the default plain-text paragraph (hosts can inject
   * markdown renderers / ChatMessageContent).
   */
  textSlot?: ReactNode;
  className?: string;
  textClassName?: string;
};

/**
 * Command chip + context chips + text (or custom text slot).
 * Renders null fragments when a field is empty.
 */
export function AiUserTurnBody({
  commandName,
  commandMeta,
  chips,
  text,
  textSlot,
  className,
  textClassName,
}: AiUserTurnBodyProps) {
  const hasChips = Boolean(chips && chips.length > 0);
  const trimmed = text?.trim() ?? "";
  const hasText = Boolean(textSlot) || trimmed.length > 0;
  if (!commandName && !hasChips && !hasText) {
    return null;
  }

  return (
    <div className={cn("altai-user-turn-body min-w-0", className)}>
      {commandName ? (
        <CommandSnippet name={commandName} meta={commandMeta} />
      ) : null}
      {hasChips ? <ContextChips chips={[...(chips ?? [])]} /> : null}
      {textSlot != null
        ? textSlot
        : trimmed
          ? (
              <p
                className={cn(
                  "whitespace-pre-wrap break-words",
                  textClassName,
                )}
              >
                {trimmed}
              </p>
            )
          : null}
    </div>
  );
}
