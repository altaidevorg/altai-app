import {
  PanelRightCloseIcon,
  PanelRightIcon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import type { ReactElement, ReactNode } from "react";
import { cn } from "../lib/cn.js";

export type WorkspaceTopbarActionsProps = {
  inspectorOpen: boolean;
  inspectorAvailable: boolean;
  onToggleInspector: () => void;
  /** Keep the Details label visible next to the icon. */
  showLabel?: boolean;
  /**
   * Host wraps compact icon controls (Desktop uses Radix tooltip). Defaults to
   * the bare control.
   */
  renderTooltip?: (label: string, children: ReactElement) => ReactNode;
};

function defaultTooltip(_label: string, children: ReactElement): ReactNode {
  return children;
}

/**
 * Run-details control for the active chat. Work / Inbox live in primary Desktop
 * navigation, not beside chat sessions.
 */
export function WorkspaceTopbarActions({
  inspectorOpen,
  inspectorAvailable,
  onToggleInspector,
  showLabel = true,
  renderTooltip = defaultTooltip,
}: WorkspaceTopbarActionsProps) {
  if (!inspectorAvailable) return null;

  const inspectorLabel = inspectorOpen ? "Close details" : "Open details";

  return (
    <div className="altai-ai-run-details-control flex shrink-0 items-center">
      {renderTooltip(
        inspectorLabel,
        <button
          type="button"
          onClick={onToggleInspector}
          aria-label={inspectorLabel}
          aria-pressed={inspectorOpen}
          title={inspectorLabel}
          className={cn(
            "inline-flex h-7 shrink-0 items-center justify-center gap-1.5 rounded-md px-1.5 text-muted-foreground transition-colors hover:bg-foreground/[0.06] hover:text-foreground",
            inspectorOpen && "bg-foreground/[0.09] text-foreground",
          )}
        >
          <HugeiconsIcon
            icon={inspectorOpen ? PanelRightCloseIcon : PanelRightIcon}
            size={14}
            strokeWidth={1.75}
          />
          {showLabel ? (
            <span className="pr-0.5 text-[10px] font-medium">Details</span>
          ) : null}
        </button>,
      )}
    </div>
  );
}
