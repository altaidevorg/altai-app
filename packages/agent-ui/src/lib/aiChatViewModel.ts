/**
 * Pure AI-SDK chat view message meta (A6.42).
 * Hosts feed status + retry outcome; no stores.
 */

import { canRetryLastAssistantTurn } from "./chatTranscriptChrome.js";
import { resolveStreamingAssistantMessageId } from "./userTurnDisplay.js";

export type AiChatViewMessageLike = {
  id: string;
  role: string;
};

export type AiChatViewRowMeta = {
  messageId: string;
  index: number;
  streaming: boolean;
  canRetry: boolean;
};

/**
 * Per-message streaming / retry flags for AiChatViewFrame maps.
 */
export function buildAiChatViewRowMeta(input: {
  messages: readonly AiChatViewMessageLike[];
  status: string;
  retryableFailure: boolean;
}): AiChatViewRowMeta[] {
  const streamingId = resolveStreamingAssistantMessageId(
    input.messages,
    input.status,
  );
  const count = input.messages.length;
  return input.messages.map((message, index) => ({
    messageId: message.id,
    index,
    streaming: message.id === streamingId,
    canRetry: canRetryLastAssistantTurn({
      retryableFailure: input.retryableFailure,
      role: message.role,
      index,
      messageCount: count,
      status: input.status,
    }),
  }));
}
