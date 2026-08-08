/**
 * Ports-first capability-gated composer follow-up bar (A6.62).
 * Package owns visibility policy; hosts supply steer/queue handlers.
 */

import { ComposerFollowupBar } from "./ComposerFollowupBar.js";
import {
  composerFollowupVisibility,
  type ComposerFollowupPolicyInput,
} from "../lib/composerFollowup.js";

export type AiComposerFollowupControlProps = {
  hasActiveRun: boolean;
  hasPrompt: boolean;
  canSteer: boolean;
  canQueue: boolean;
  /** When false, treat as cannot start new work (default true). */
  canStartRun?: boolean;
  onSteer: () => void;
  onQueue: () => void;
  steerTitle?: string;
  queueTitle?: string;
};

/**
 * Returns null when follow-up bar should not mount.
 */
export function AiComposerFollowupControl({
  hasActiveRun,
  hasPrompt,
  canSteer,
  canQueue,
  canStartRun = true,
  onSteer,
  onQueue,
  steerTitle = "Apply at the active run's next safe boundary",
  queueTitle = "Start after the active run terminates",
}: AiComposerFollowupControlProps) {
  const input: ComposerFollowupPolicyInput = {
    hasActiveRun,
    canStartRun,
    canSteer,
    canQueue,
    hasPrompt,
  };
  const visibility = composerFollowupVisibility(input);
  if (!visibility.showBar) {
    return null;
  }
  return (
    <ComposerFollowupBar
      hint={visibility.hint}
      showSteer={visibility.showSteer}
      showQueue={visibility.showQueue}
      canSteer={visibility.canSteerAction}
      canQueue={visibility.canQueueAction}
      onSteer={onSteer}
      onQueue={onQueue}
      steerTitle={steerTitle}
      queueTitle={queueTitle}
    />
  );
}
