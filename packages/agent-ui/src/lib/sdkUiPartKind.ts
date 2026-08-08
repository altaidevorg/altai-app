/**
 * Pure AI-SDK UI-part kind classification (A6.44).
 * Hosts map kinds to MessageResponse / Reasoning / tool renderers.
 */

import {
  isSdkToolPart,
  type SdkToolPartLike,
} from "./sdkToolPartMap.js";

export type SdkUiPartKind = "text" | "reasoning" | "tool" | "unknown";

export type SdkUiPartLike = {
  type?: string;
  text?: string;
} & SdkToolPartLike;

export function classifySdkUiPart(part: SdkUiPartLike): SdkUiPartKind {
  if (part.type === "text") return "text";
  if (part.type === "reasoning") return "reasoning";
  if (isSdkToolPart(part)) return "tool";
  return "unknown";
}

export function sdkPartText(part: SdkUiPartLike): string {
  return typeof part.text === "string" ? part.text : "";
}
