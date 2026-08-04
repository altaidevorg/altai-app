import { Tick02Icon } from "@hugeicons/core-free-icons";
import { SurfaceEmptyState } from "./AuxiliarySurface.js";

/**
 * Default empty state for the notification inbox. A domain-specific preset
 * of `SurfaceEmptyState` with the "all caught up" message.
 */
export function EmptyInbox() {
  return (
    <SurfaceEmptyState
      icon={Tick02Icon}
      title="You're all caught up"
      description="Questions, review-ready results, and durable agent updates will appear here."
      className="border-0 bg-transparent"
    />
  );
}
