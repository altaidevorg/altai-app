/**
 * Ports-first assistant stop/retry footer switch (A6.50).
 * Package owns mode resolution; hosts supply action chrome.
 */

import type { ReactNode } from "react";
import {
  resolveAssistantRunActionMode,
  type AssistantRunActionMode,
} from "../lib/chatSdkAssistantChrome.js";

export type AiAssistantRunActionsProps = {
  streaming: boolean;
  canRetry?: boolean;
  renderStop: () => ReactNode;
  renderRetry: () => ReactNode;
  /** Optional wrapper for the action row; default unwraps children. */
  wrap?: (children: ReactNode, mode: Exclude<AssistantRunActionMode, "hidden">) => ReactNode;
};

export function AiAssistantRunActions({
  streaming,
  canRetry,
  renderStop,
  renderRetry,
  wrap,
}: AiAssistantRunActionsProps): ReactNode {
  const mode = resolveAssistantRunActionMode({ streaming, canRetry });
  if (mode === "hidden") return null;
  const body = mode === "stop" ? renderStop() : renderRetry();
  return wrap ? wrap(body, mode) : body;
}
