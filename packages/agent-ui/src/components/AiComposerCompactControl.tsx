/**
 * Ports-first capability-gated compact control (A6.61).
 * Package owns mount/disable policy; hosts supply onCompact transport.
 */

import { CompactNowControl } from "./CompactNowControl.js";
import {
  canInvokeCompact,
  canMountCompactControl,
  type ComposerCompactFlags,
} from "../lib/composerCompactPolicy.js";

export type AiComposerCompactControlProps = {
  canCompact: boolean;
  hasActiveChat: boolean;
  busy?: boolean;
  onCompact: () => void;
};

/**
 * Returns null when capability or active chat is missing (no dead placeholders).
 */
export function AiComposerCompactControl({
  canCompact,
  hasActiveChat,
  busy = false,
  onCompact,
}: AiComposerCompactControlProps) {
  const flags: ComposerCompactFlags = {
    canCompact,
    hasActiveChat,
    busy,
  };
  if (!canMountCompactControl(flags)) {
    return null;
  }
  return (
    <CompactNowControl
      disabled={!canInvokeCompact(flags)}
      onClick={onCompact}
    />
  );
}
