import { Add01Icon, Cancel01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import type { ReactElement, ReactNode } from "react";
import { cn } from "../lib/cn.js";

export type ChatTabItem = {
  id: string;
  title: string;
};

export type ChatTabStripProps = {
  tabs: ChatTabItem[];
  activeId: string | null;
  embedded?: boolean;
  onSelect: (id: string) => void;
  onClose: (id: string) => void;
  onNewChat: () => void;
  /**
   * Host wraps compact icon controls (Desktop uses Radix tooltip). Defaults to
   * the bare control so VS Code can rely on native titles.
   */
  renderTooltip?: (label: string, children: ReactElement) => ReactNode;
};

function defaultTooltip(_label: string, children: ReactElement): ReactNode {
  return children;
}

/**
 * Open-chat tab strip with a trailing new-chat action. Purely presentational;
 * the host supplies resolved tabs and selection/close transport.
 */
export function ChatTabStrip({
  tabs,
  activeId,
  embedded = false,
  onSelect,
  onClose,
  onNewChat,
  renderTooltip = defaultTooltip,
}: ChatTabStripProps) {
  return (
    <div
      className={cn(
        "altai-ai-chat-tabs flex h-10 min-w-0 items-center gap-1.5",
        embedded
          ? "flex-1 bg-transparent"
          : "shrink-0 border-b border-border-subtle bg-card px-2.5",
      )}
    >
      <div
        role="tablist"
        aria-label="Open chats"
        className="flex min-w-0 shrink items-center gap-1 overflow-x-auto"
      >
        {tabs.map((session) => {
          const title = session.title || "New chat";
          const closeLabel = `Close ${session.title || "new chat"}`;
          return (
            <div
              key={session.id}
              className={cn(
                "group flex h-7 max-w-44 shrink-0 items-center rounded-lg border text-[10.5px] transition-colors",
                session.id === activeId
                  ? "border-border bg-muted/70 font-medium text-foreground"
                  : "border-transparent text-muted-foreground hover:border-border/60 hover:bg-accent hover:text-foreground",
              )}
            >
              <button
                id={`altai-chat-tab-${session.id}`}
                type="button"
                role="tab"
                aria-controls="altai-active-chat"
                aria-selected={session.id === activeId}
                onClick={() => onSelect(session.id)}
                title={title}
                className="h-full min-w-0 truncate px-2.5 text-left outline-none"
              >
                {title}
              </button>
              {renderTooltip(
                closeLabel,
                <button
                  type="button"
                  onClick={() => onClose(session.id)}
                  aria-label={closeLabel}
                  title={closeLabel}
                  className="mr-1 inline-flex size-4 shrink-0 items-center justify-center rounded-md text-muted-foreground/70 hover:bg-foreground/[0.1] hover:text-foreground"
                >
                  <HugeiconsIcon icon={Cancel01Icon} size={10} strokeWidth={2} />
                </button>,
              )}
            </div>
          );
        })}
      </div>
      {renderTooltip(
        "New chat",
        <button
          type="button"
          onClick={onNewChat}
          aria-label="New chat"
          title="New chat"
          className="inline-flex size-7 shrink-0 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-foreground/[0.06] hover:text-foreground"
        >
          <HugeiconsIcon icon={Add01Icon} size={14} strokeWidth={1.75} />
        </button>,
      )}
    </div>
  );
}
