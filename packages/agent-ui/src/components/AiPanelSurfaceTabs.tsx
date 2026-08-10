import { cn } from "../lib/cn.js";

export type AiPanelSurfaceTab = {
  id: string;
  label: string;
};

export type AiPanelSurfaceTabsProps = {
  /** The currently visible panel surface. */
  activeId: string;
  /** Ordered set of navigable host surfaces. */
  tabs: readonly AiPanelSurfaceTab[];
  /** Hosts own surface state and routing. */
  onSelect: (id: string) => void;
  className?: string;
  "aria-label"?: string;
};

/**
 * Shared text-tab navigation for hosts that expose panel surfaces below the
 * main chrome. It deliberately owns no routing or capability state.
 */
export function AiPanelSurfaceTabs({
  activeId,
  tabs,
  onSelect,
  className,
  "aria-label": ariaLabel = "AI panel surfaces",
}: AiPanelSurfaceTabsProps) {
  return (
    <div
      className={cn("altai-view-tabs", className)}
      role="tablist"
      aria-label={ariaLabel}
      data-ai-panel-surface-tabs
    >
      {tabs.map((tab) => (
        <button
          key={tab.id}
          type="button"
          role="tab"
          aria-selected={activeId === tab.id}
          className="altai-view-tab"
          onClick={() => onSelect(tab.id)}
        >
          {tab.label}
        </button>
      ))}
    </div>
  );
}
