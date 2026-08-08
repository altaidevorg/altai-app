/**
 * Pure AI-SDK assistant group prep (A6.49).
 * Hosts bind streaming + render after this snapshot.
 */

import {
  buildTranscriptPartGroups,
  type ToolLikePart,
  type TranscriptPartGroup,
} from "./transcriptToolGroups.js";
import { indexOfLastTextPart } from "./userTurnDisplay.js";

export type AssistantSdkGroupsState<T = ToolLikePart> = {
  lastTextPartIdx: number;
  groups: TranscriptPartGroup<T>[];
};

export function buildAssistantSdkGroupsState<T extends { type?: string }>(
  parts: readonly T[],
): AssistantSdkGroupsState<T> {
  return {
    lastTextPartIdx: indexOfLastTextPart(parts),
    groups: buildTranscriptPartGroups(parts as T[]),
  };
}
