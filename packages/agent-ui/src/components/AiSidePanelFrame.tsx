import { forwardRef, type ReactNode } from "react";
import { cn } from "../lib/cn.js";
import type { SidePanelVariant } from "../lib/sidePanelLayout.js";

export type AiSidePanelFrameProps = {
  /** History / tabs / Work / Inbox / settings chrome. */
  topbar: ReactNode;
  /** Main column body (chat + composer, history overlay, inspector overlay…). */
  children: ReactNode;
  variant?: SidePanelVariant;
  className?: string;
  /** Accessible name for the panel landmark. */
  "aria-label"?: string;
  id?: string;
};

/**
 * Outer side-chat frame: panel root + optional topbar + body column.
 * Hosts inject Desktop WorkspaceTopbar / VS Code ChatShell chrome and the
 * full main/overlay tree. No stores, Tauri, or HostPorts.
 */
export const AiSidePanelFrame = forwardRef<HTMLElement, AiSidePanelFrameProps>(
  function AiSidePanelFrame(
    {
      topbar,
      children,
      variant = "sidebar",
      className,
      "aria-label": ariaLabel,
      id = "altai-ai-panel",
    },
    ref,
  ) {
    return (
      <aside
        ref={ref}
        data-ai-side-panel
        data-ai-workspace={variant === "workspace" ? "true" : undefined}
        id={id}
        aria-label={
          ariaLabel ??
          (variant === "workspace" ? "ALTAI agent workspace" : "AI assistant")
        }
        className={cn(
          "altai-ai-panel @container relative flex h-full min-h-0 overflow-hidden bg-card text-[12px]",
          className,
        )}
      >
        <div className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
          {topbar}
          {children}
        </div>
      </aside>
    );
  },
);
