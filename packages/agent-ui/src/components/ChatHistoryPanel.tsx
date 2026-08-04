import { Add01Icon, Search01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import type { RefObject } from "react";
import type { SessionHistoryGroup } from "../lib/sessionHistory.js";
import { SessionRow } from "./SessionRow.js";

export type ChatHistoryPanelProps = {
  groups: SessionHistoryGroup[];
  activeId: string | null;
  search: string;
  onSearchChange: (value: string) => void;
  onNewChat: () => void;
  onPick: (id: string) => void;
  onDelete: (id: string) => void;
  renamingId: string | null;
  renameValue: string;
  onStartRename: (id: string, title: string) => void;
  onCommitRename: () => void;
  onCancelRename: () => void;
  onRenameValueChange: (value: string) => void;
  renameInputRef: RefObject<HTMLInputElement | null>;
  searchInputRef?: RefObject<HTMLInputElement | null>;
};

/**
 * Inline chat history surface: new-chat action, search, and sessions grouped
 * by recency. Host owns filtering, snippet loading, and session mutations.
 */
export function ChatHistoryPanel({
  groups,
  activeId,
  search,
  onSearchChange,
  onNewChat,
  onPick,
  onDelete,
  renamingId,
  renameValue,
  onStartRename,
  onCommitRename,
  onCancelRename,
  onRenameValueChange,
  renameInputRef,
  searchInputRef,
}: ChatHistoryPanelProps) {
  return (
    <div className="altai-ai-history flex min-h-0 flex-1 flex-col bg-card">
      <div className="flex shrink-0 flex-col gap-2 border-b border-border-subtle px-2.5 py-2">
        <div className="flex items-center gap-1.5">
          <button
            type="button"
            onClick={onNewChat}
            className="altai-ai-history-new flex flex-1 items-center justify-center gap-1.5 bg-foreground/[0.07] px-2 py-1.5 text-[11.5px] font-medium text-foreground transition-colors hover:bg-foreground/[0.12]"
          >
            <HugeiconsIcon icon={Add01Icon} size={13} strokeWidth={2} />
            New chat
          </button>
        </div>
        <div className="altai-ai-history-search flex items-center gap-2 border border-border bg-muted px-2">
          <HugeiconsIcon
            icon={Search01Icon}
            size={13}
            strokeWidth={1.75}
            className="shrink-0 text-muted-foreground/80"
          />
          <input
            ref={searchInputRef}
            aria-label="Search chat history"
            value={search}
            onChange={(e) => onSearchChange(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Escape" && search) {
                e.stopPropagation();
                onSearchChange("");
              }
            }}
            placeholder="Search chat history…"
            className="w-full bg-transparent py-1.5 text-[12px] outline-none placeholder:text-muted-foreground/60"
          />
        </div>
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto py-1">
        {groups.length === 0 ? (
          <div className="px-3 py-8 text-center text-[11px] text-muted-foreground/70">
            {search ? "No chats match." : "No chats yet."}
          </div>
        ) : (
          groups.map((group) => (
            <div key={group.label} className="px-1">
              <div className="px-2 pb-1 pt-2 text-[10px] font-medium uppercase tracking-wide text-muted-foreground/80">
                {group.label}
              </div>
              {group.items.map((session) => {
                const title = session.title || "New chat";
                const renaming = renamingId === session.id;
                return (
                  <div key={session.id} className="altai-ai-history-row">
                    <SessionRow
                      title={title}
                      snippet={session.snippet}
                      active={session.id === activeId}
                      renaming={renaming}
                      renameValue={renameValue}
                      onPick={() => onPick(session.id)}
                      onStartRename={() => onStartRename(session.id, title)}
                      onCommitRename={onCommitRename}
                      onCancelRename={onCancelRename}
                      onRenameValueChange={onRenameValueChange}
                      onDelete={() => onDelete(session.id)}
                      renameInputRef={renameInputRef}
                    />
                  </div>
                );
              })}
            </div>
          ))
        )}
      </div>
    </div>
  );
}
