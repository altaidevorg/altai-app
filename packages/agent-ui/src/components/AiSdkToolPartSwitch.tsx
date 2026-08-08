/**
 * Ports-first AI-SDK tool part switch (A6.47).
 * Package maps approval vs card views; hosts own approval UI and Tool chrome.
 */

import type { ReactNode } from "react";
import {
  mapSdkToolApprovalPart,
  mapSdkToolCardPart,
  type SdkToolApprovalView,
  type SdkToolCardView,
  type SdkToolPartLike,
} from "../lib/sdkToolPartMap.js";

export type AiSdkToolPartSwitchProps = {
  part: SdkToolPartLike;
  renderApproval: (view: SdkToolApprovalView) => ReactNode;
  renderCard: (view: SdkToolCardView) => ReactNode;
};

export function AiSdkToolPartSwitch({
  part,
  renderApproval,
  renderCard,
}: AiSdkToolPartSwitchProps): ReactNode {
  const approval = mapSdkToolApprovalPart(part);
  if (approval) {
    return renderApproval(approval);
  }
  return renderCard(mapSdkToolCardPart(part));
}
