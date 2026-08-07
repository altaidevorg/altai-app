import type { ReactNode } from "react";
import { cn } from "../lib/cn.js";

export type AiChatMainColumnProps = {
  /**
   * Optional plan-mode strip above the transcript (Desktop PlanModeStrip;
   * VS Code plan/todo chrome).
   */
  planMode?: ReactNode;
  /**
   * Primary scroll region: empty home or message list / AiChatView.
   * Hosts own scroll containers and message mapping.
   */
  transcript: ReactNode;
  /**
   * Between transcript and composer: recovery, clarification, change-review
   * banners, run status strips, interactive prompts.
   */
  runChrome?: ReactNode;
  /** Composer dock (AiInputBar / shared AiComposer host). */
  composer: ReactNode;
  /** Under composer: project target, connect banners, etc. */
  footer?: ReactNode;
  className?: string;
  /** Landmark id (default Desktop-compatible). */
  id?: string;
  "aria-label"?: string;
};

/**
 * Shared active-chat column frame used by Desktop `AiSidePanel` Body and the
 * VS Code chat surface. Slot-only: no stores, events, or HostPorts.
 *
 * Order matches Desktop density:
 * planMode → transcript → runChrome → composer → footer.
 */
export function AiChatMainColumn({
  planMode,
  transcript,
  runChrome,
  composer,
  footer,
  className,
  id = "altai-active-chat",
  "aria-label": ariaLabel = "Active chat session",
}: AiChatMainColumnProps) {
  return (
    <div
      id={id}
      role="tabpanel"
      aria-label={ariaLabel}
      tabIndex={-1}
      className={cn(
        "altai-ai-chat-main flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden",
        className,
      )}
    >
      {planMode}
      <div className="altai-ai-chat-transcript flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
        {transcript}
      </div>
      {runChrome}
      {composer}
      {footer}
    </div>
  );
}
