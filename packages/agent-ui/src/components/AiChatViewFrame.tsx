/**
 * Ports-first AI-SDK chat transcript frame (A6.42).
 * Hosts own Conversation shell, message bubbles (Message/Tool), and stores.
 */

import type { ReactNode } from "react";
import { useMemo } from "react";
import { AiChatTranscriptFrame } from "./AiChatTranscriptFrame.js";
import { TranscriptConversationEmpty } from "./TranscriptConversationEmpty.js";
import { TranscriptRunError } from "./TranscriptRunError.js";
import {
  resolveChatAriaLive,
  resolveTranscriptRunErrorVariant,
} from "../lib/chatTranscriptChrome.js";
import {
  buildAiChatViewRowMeta,
  type AiChatViewMessageLike,
} from "../lib/aiChatViewModel.js";

export type AiChatViewFrameProps<M extends AiChatViewMessageLike> = {
  messages: readonly M[];
  /** AI SDK chat status string (`idle` | `streaming` | …). */
  status: string;
  /** Host announce preference (`off` | `polite` | `assertive`). */
  announce?: string;
  retryableFailure?: boolean;
  error?: { message: string } | null;
  onDismissError?: () => void;
  /** Live status inside empty transcript (e.g. AgentStatusPill). */
  emptyStatus?: ReactNode;
  /** Live status at end of filled transcript. */
  endStatus?: ReactNode;
  renderMessage: (input: {
    message: M;
    index: number;
    streaming: boolean;
    canRetry: boolean;
  }) => ReactNode;
  /**
   * Outer scroll shell. Desktop wraps Conversation; VS Code can use a region.
   * `body` already includes AiChatTranscriptFrame.
   */
  renderRoot: (input: {
    "aria-live": ReturnType<typeof resolveChatAriaLive>;
    isEmpty: boolean;
    body: ReactNode;
  }) => ReactNode;
};

/**
 * Store-free chat transcript: frame + empty/error chrome + message meta.
 */
export function AiChatViewFrame<M extends AiChatViewMessageLike>({
  messages,
  status,
  announce = "polite",
  retryableFailure = false,
  error = null,
  onDismissError,
  emptyStatus,
  endStatus,
  renderMessage,
  renderRoot,
}: AiChatViewFrameProps<M>) {
  const ariaLive = resolveChatAriaLive(announce);
  const rows = useMemo(
    () =>
      buildAiChatViewRowMeta({
        messages,
        status,
        retryableFailure,
      }),
    [messages, status, retryableFailure],
  );
  const isEmpty = messages.length === 0;

  const body = isEmpty ? (
    <AiChatTranscriptFrame
      isEmpty
      aria-live={ariaLive}
      empty={
        <TranscriptConversationEmpty>{emptyStatus}</TranscriptConversationEmpty>
      }
    />
  ) : (
    <AiChatTranscriptFrame
      isEmpty={false}
      aria-live={ariaLive}
      end={
        <>
          {endStatus}
          {error ? (
            <TranscriptRunError
              message={error.message}
              variant={resolveTranscriptRunErrorVariant(error.message)}
              onDismiss={onDismissError}
            />
          ) : null}
        </>
      }
    >
      {messages.map((message, index) => {
        const meta = rows[index]!;
        return renderMessage({
          message,
          index,
          streaming: meta.streaming,
          canRetry: meta.canRetry,
        });
      })}
    </AiChatTranscriptFrame>
  );

  return renderRoot({ "aria-live": ariaLive, isEmpty, body });
}
