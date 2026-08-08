/**
 * Ports-first flat display-message action footer cluster (A6.58).
 * Flags gate which slots render; hosts supply button chrome + handlers.
 */

import type { ReactNode } from "react";
import type { DisplayMessageActionFlags } from "../lib/chatDisplayActions.js";
import { hasDisplayMessageActions } from "../lib/chatDisplayActions.js";

export type AiDisplayMessageActionsProps = {
  flags: DisplayMessageActionFlags;
  copy?: ReactNode;
  edit?: ReactNode;
  retry?: ReactNode;
  openDiff?: ReactNode;
  openFile?: ReactNode;
  /** Extra trailing actions (host-specific). */
  extra?: ReactNode;
};

/**
 * Returns null when no flags are set. Does not wrap in footer chrome —
 * `AiDisplayMessageBubble` already places this in `actions`.
 */
export function AiDisplayMessageActions({
  flags,
  copy,
  edit,
  retry,
  openDiff,
  openFile,
  extra,
}: AiDisplayMessageActionsProps): ReactNode {
  if (!hasDisplayMessageActions(flags)) {
    return null;
  }
  return (
    <>
      {flags.showCopy ? copy : null}
      {flags.showEdit ? edit : null}
      {flags.showRetry ? retry : null}
      {flags.showOpenDiff ? openDiff : null}
      {flags.showOpenFile ? openFile : null}
      {extra}
    </>
  );
}
