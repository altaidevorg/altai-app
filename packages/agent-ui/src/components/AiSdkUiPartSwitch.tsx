/**
 * Ports-first AI-SDK UI part switcher (A6.46).
 * Package routes mapSdkUiPartView; hosts supply text / reasoning / tool chrome.
 */

import type { ReactNode } from "react";
import {
  mapSdkUiPartView,
  type SdkUiPartView,
} from "../lib/sdkUiPartView.js";
import type { SdkUiPartLike } from "../lib/sdkUiPartKind.js";
import type { SdkToolPartLike } from "../lib/sdkToolPartMap.js";

export type AiSdkUiPartSwitchProps = {
  part: SdkUiPartLike;
  streaming?: boolean;
  renderText: (text: string, streaming: boolean) => ReactNode;
  renderReasoning: (text: string) => ReactNode;
  renderTool: (part: SdkToolPartLike) => ReactNode;
  /** Optional host hook for unknown kinds; default null. */
  renderUnknown?: (view: Extract<SdkUiPartView, { kind: "unknown" }>) => ReactNode;
};

export function AiSdkUiPartSwitch({
  part,
  streaming = false,
  renderText,
  renderReasoning,
  renderTool,
  renderUnknown,
}: AiSdkUiPartSwitchProps): ReactNode {
  const view = mapSdkUiPartView(part);
  if (view.kind === "text") {
    return renderText(view.text, streaming);
  }
  if (view.kind === "reasoning") {
    return renderReasoning(view.text);
  }
  if (view.kind === "tool") {
    return renderTool(view.part);
  }
  return renderUnknown?.(view) ?? null;
}
