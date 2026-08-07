import { useEffect, useRef, useState, type ReactNode } from "react";
import { cn } from "../lib/cn.js";

export type AgentChatLayoutDensity = "sidebar" | "desktop" | "auto";

export type AgentChatLayoutProps = {
  /** History column or stacked sessions surface. */
  history?: ReactNode;
  /** Primary chat column (messages + composer). */
  main: ReactNode;
  /**
   * `sidebar` = column stack for narrow VS Code Activity Bar.
   * `desktop` = history rail beside main (Desktop / wide secondary sidebar).
   * `auto` = switch at ~36rem container width (default for VS Code host).
   */
  density?: AgentChatLayoutDensity;
  className?: string;
  /** px width (container) at which `auto` becomes `desktop`. Default 576 (36rem). */
  autoDesktopMinWidth?: number;
};

/**
 * Shared outer split for chat: history + main. Hosts supply content slots only.
 */
export function AgentChatLayout({
  history,
  main,
  density = "auto",
  className,
  autoDesktopMinWidth = 576,
}: AgentChatLayoutProps) {
  const rootRef = useRef<HTMLDivElement | null>(null);
  const [resolved, setResolved] = useState<"sidebar" | "desktop">(() =>
    density === "desktop" ? "desktop" : "sidebar",
  );

  useEffect(() => {
    if (density === "desktop" || density === "sidebar") {
      setResolved(density);
      return;
    }
    const el = rootRef.current;
    if (!el) {
      return;
    }
    const apply = (width: number) => {
      setResolved(width >= autoDesktopMinWidth ? "desktop" : "sidebar");
    };
    apply(el.getBoundingClientRect().width);
    if (typeof ResizeObserver === "undefined") {
      return;
    }
    const observer = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (!entry) {
        return;
      }
      apply(entry.contentRect.width);
    });
    observer.observe(el);
    return () => {
      observer.disconnect();
    };
  }, [density, autoDesktopMinWidth]);

  const hasHistory = history != null && history !== false;
  return (
    <div
      ref={rootRef}
      className={cn(
        "altai-agent-chat-layout flex min-h-0 flex-1",
        resolved === "desktop" ? "flex-row" : "flex-col",
        className,
      )}
      data-density={resolved}
      data-density-requested={density}
    >
      {hasHistory ? (
        <div
          className={cn(
            "altai-agent-chat-history flex min-h-0 min-w-0",
            resolved === "desktop"
              ? "w-[11.5rem] max-w-56 shrink-0 flex-col border-r border-border-subtle"
              : "w-full shrink-0 flex-col border-b border-border-subtle",
          )}
        >
          {history}
        </div>
      ) : null}
      <div className="altai-agent-chat-main flex min-h-0 min-w-0 flex-1 flex-col">
        {main}
      </div>
    </div>
  );
}
