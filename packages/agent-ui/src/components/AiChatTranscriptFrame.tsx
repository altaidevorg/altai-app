import type { ReactNode } from "react";
import { cn } from "../lib/cn.js";
import type { resolveChatAriaLive } from "../lib/chatTranscriptChrome.js";

export type AiChatTranscriptFrameProps = {
  /** True when no transcript messages — shows `empty` instead of children. */
  isEmpty: boolean;
  empty?: ReactNode;
  /** Message list / bubbles. */
  children?: ReactNode;
  /**
   * Status pill + run error strip at end of filled transcript (Desktop places
   * these inside the scroll region).
   */
  end?: ReactNode;
  /** From `resolveChatAriaLive`. */
  "aria-live"?: ReturnType<typeof resolveChatAriaLive>;
  className?: string;
  /**
   * Content-box className for filled transcripts (default matches Desktop
   * max-width density).
   */
  contentClassName?: string;
};

/**
 * Inner transcript body frame for AiChatView / ChatMessageList hosts.
 * Hosts own the outer scroll container (Desktop Conversation, VS Code region).
 * Slot-only: no stores or HostPorts.
 */
export function AiChatTranscriptFrame({
  isEmpty,
  empty,
  children,
  end,
  "aria-live": ariaLive = "polite",
  className,
  contentClassName,
}: AiChatTranscriptFrameProps) {
  if (isEmpty) {
    return (
      <div
        className={cn(
          "altai-ai-transcript-frame altai-ai-transcript-empty min-w-0",
          className,
        )}
        aria-live={ariaLive}
      >
        {empty}
      </div>
    );
  }
  return (
    <div
      className={cn(
        "altai-ai-transcript-frame min-w-0",
        className,
      )}
      aria-live={ariaLive}
    >
      <div
        className={cn(
          "altai-ai-transcript mx-auto flex min-w-0 w-full max-w-[52rem] flex-col gap-5 px-4 py-5 @[44rem]:px-6",
          contentClassName,
        )}
      >
        {children}
        {end}
      </div>
    </div>
  );
}
