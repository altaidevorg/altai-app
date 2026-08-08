/**
 * Pure AI-SDK UI-part → host render view (A6.45).
 * Hosts own MessageResponse / Reasoning / Tool chrome; package owns routing.
 */

import {
  classifySdkUiPart,
  sdkPartText,
  type SdkUiPartLike,
} from "./sdkUiPartKind.js";
import type { SdkToolPartLike } from "./sdkToolPartMap.js";

export type SdkUiPartTextView = {
  kind: "text";
  text: string;
};

export type SdkUiPartReasoningView = {
  kind: "reasoning";
  text: string;
};

export type SdkUiPartToolView = {
  kind: "tool";
  part: SdkToolPartLike;
};

export type SdkUiPartUnknownView = {
  kind: "unknown";
};

export type SdkUiPartView =
  | SdkUiPartTextView
  | SdkUiPartReasoningView
  | SdkUiPartToolView
  | SdkUiPartUnknownView;

/** Map an AI-SDK UI part into a host-renderable view. */
export function mapSdkUiPartView(part: SdkUiPartLike): SdkUiPartView {
  const kind = classifySdkUiPart(part);
  if (kind === "text") {
    return { kind: "text", text: sdkPartText(part) };
  }
  if (kind === "reasoning") {
    return { kind: "reasoning", text: sdkPartText(part) };
  }
  if (kind === "tool") {
    return { kind: "tool", part };
  }
  return { kind: "unknown" };
}
