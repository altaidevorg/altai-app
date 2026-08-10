import type { ReactNode } from "react";
import { cn } from "../lib/cn.js";

export type AiPanelTopbarProps = {
  /** Primary row: tabs, host status, and close/settings controls. */
  primary: ReactNode;
  /** Optional secondary row for panel-level destinations and status. */
  secondary?: ReactNode;
  className?: string;
  "aria-label"?: string;
};

/**
 * Shared structural frame for the Desktop and VS Code side-panel topbars.
 * Hosts supply their controls and transport callbacks; this component owns
 * only the stable panel chrome boundary and its accessibility label.
 */
export function AiPanelTopbar({
  primary,
  secondary,
  className,
  "aria-label": ariaLabel = "AI panel chrome",
}: AiPanelTopbarProps) {
  return (
    <header
      data-ai-panel-topbar
      aria-label={ariaLabel}
      className={cn(
        "altai-ai-topbar flex shrink-0 flex-col border-b border-border-subtle bg-card",
        className,
      )}
    >
      {primary}
      {secondary}
    </header>
  );
}
