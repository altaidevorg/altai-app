import {
  Conversation,
  ConversationContent,
  ConversationScrollButton,
} from "@/components/ai-elements/conversation";
import {
  Reasoning,
  ReasoningContent,
  ReasoningTrigger,
} from "@/components/ai-elements/reasoning";
import { Tool } from "@/components/ai-elements/tool";
import { motion } from "motion/react";
import { HugeiconsIcon } from "@hugeicons/react";
import {
  Cancel01Icon,
  File01Icon,
  GlobalSearchIcon,
  Refresh01Icon,
  TerminalIcon,
} from "@hugeicons/core-free-icons";
import { resolveSlashCommand } from "../lib/slashCommands";
import { usePreferencesStore } from "@/modules/settings/preferences";
import { retryFailedRun, useChatStore } from "../store/chatStore";
import { useAgentRunsStore } from "../store/agentRunsStore";
import {
  isRetryableRunOutcome,
} from "../lib/agentEventBridge";
import type {
  ChatStatus,
  DynamicToolUIPart,
  ToolUIPart,
  UIMessage,
  UIMessagePart,
} from "ai";
import { memo, useCallback, useMemo } from "react";
import {
  AiChatTranscriptFrame,
  AiSdkAssistantGroups,
  AiToolApproval,
  AssistantBrandLabel,
  AiUserTurnBody,
  buildTranscriptPartGroups,
  canRetryLastAssistantTurn,
  HoverActionButton,
  indexOfLastTextPart,
  joinMessageTextParts,
  prepareUserTurnDisplay,
  resolveChatAriaLive,
  resolveStreamingAssistantMessageId,
  resolveTranscriptRunErrorVariant,
  shouldShowAssistantRunActions,
  TranscriptConversationEmpty,
  TranscriptRunError,
} from "@altai/agent-ui";
import { AgentStatusPill } from "./AgentStatusPill";
import { openWorkspaceFile } from "../lib/openChatHref";
import {
  Message,
  MessageActions,
  MessageContent,
  MessageResponse,
} from "@/components/ai-elements/message";

type AnyToolPart = ToolUIPart | DynamicToolUIPart;


type AnyPart = UIMessagePart<Record<string, never>, Record<string, never>>;

type ApprovalArg = {
  id: string;
  approved: boolean;
  reason?: string;
};

type Props = {
  messages: UIMessage[];
  status: ChatStatus;
  error: Error | undefined;
  clearError: () => void;
  addToolApprovalResponse: (arg: ApprovalArg) => void | PromiseLike<void>;
  stop?: () => void;
};

export function AiChatView({
  messages,
  status,
  error,
  clearError,
  addToolApprovalResponse,
  stop,
}: Props) {
  // Accessibility — pref-driven aria-live policy for the chat transcript.
  // "off" disables announcements entirely (some SR users prefer to pull
  // updates via virtual cursor instead of being interrupted on every chunk).
  const chatAnnounce = usePreferencesStore((s) => s.chatAnnounce);
  const activeSessionId = useChatStore((s) => s.activeSessionId);
  const retryableFailure = useAgentRunsStore((s) => {
    if (!activeSessionId) return false;
    const outcome = s.runs[activeSessionId]?.outcome;
    return isRetryableRunOutcome(outcome);
  });
  const ariaLiveProp = resolveChatAriaLive(chatAnnounce);
  const streamingMessageId = resolveStreamingAssistantMessageId(
    messages,
    status,
  );

  const onApproval = useCallback(
    (id: string, approved: boolean) => addToolApprovalResponse({ id, approved }),
    [addToolApprovalResponse],
  );

  if (messages.length === 0) {
    return (
      <Conversation className="altai-ai-conversation overflow-x-hidden" aria-live={ariaLiveProp}>
        <ConversationContent className="min-w-0">
          <AiChatTranscriptFrame
            isEmpty
            aria-live={ariaLiveProp}
            empty={
              <TranscriptConversationEmpty>
                {/* Live status stays inside the transcript viewport, not above the composer. */}
                <AgentStatusPill hideError />
              </TranscriptConversationEmpty>
            }
          />
        </ConversationContent>
      </Conversation>
    );
  }

  return (
    <Conversation className="altai-ai-conversation overflow-x-hidden" aria-live={ariaLiveProp}>
      <ConversationContent className="min-w-0">
        <AiChatTranscriptFrame
          isEmpty={false}
          aria-live={ariaLiveProp}
          end={
            <>
              {/* Agent working indicator — end of transcript, inside the chat scroll. */}
              <AgentStatusPill hideError />
              {error ? (
                <TranscriptRunError
                  message={error.message}
                  variant={resolveTranscriptRunErrorVariant(error.message)}
                  onDismiss={clearError}
                />
              ) : null}
            </>
          }
        >
          {messages.map((m, i) => (
            <RenderedMessage
              key={m.id}
              message={m}
              onApproval={onApproval}
              streaming={m.id === streamingMessageId}
              canRetry={canRetryLastAssistantTurn({
                retryableFailure,
                role: m.role,
                index: i,
                messageCount: messages.length,
                status,
              })}
              onRetry={() => void retryFailedRun()}
              onStop={() => void stop?.()}
            />
          ))}
        </AiChatTranscriptFrame>
      </ConversationContent>
      <ConversationScrollButton />
    </Conversation>
  );
}

const RenderedMessage = memo(function RenderedMessage({
  message,
  onApproval,
  streaming,
  canRetry,
  onRetry,
  onStop,
}: {
  message: UIMessage;
  onApproval: (id: string, approved: boolean) => void;
  streaming: boolean;
  canRetry?: boolean;
  onRetry?: () => void;
  onStop?: () => void;
}) {
  // Index of the trailing text part — only that one is "live" mid-stream.
  // Earlier text parts (separated by tool calls) are already finalized.
  const lastTextIdx = indexOfLastTextPart(message.parts);
  if (message.role === "user") {
    const rawText = joinMessageTextParts(message.parts);

    const stripped = prepareUserTurnDisplay(rawText);

    return (
      <Message from="user" className="altai-ai-message">
        <MessageContent>
          <AiUserTurnBody
            commandName={stripped.commandName}
            commandMeta={
              stripped.commandName
                ? (() => {
                    const meta = resolveSlashCommand(stripped.commandName);
                    return meta
                      ? {
                          invocation: meta.invocation,
                          label: meta.label,
                          icon: (
                            <HugeiconsIcon
                              icon={meta.icon}
                              size={12}
                              strokeWidth={1.75}
                              className="shrink-0 text-foreground"
                            />
                          ),
                        }
                      : null;
                  })()
                : null
            }
            chips={stripped.chips}
            text={stripped.text}
          />
        </MessageContent>
      </Message>
    );
  }

  const groups = useMemo(
    () => buildTranscriptPartGroups(message.parts as AnyPart[]),
    [message.parts],
  );

  const showRunActions = shouldShowAssistantRunActions({
    streaming,
    canRetry,
  });

  return (
    <Message from={message.role} className="altai-ai-message">
      <MessageContent>
        <AssistantBrandLabel streaming={streaming} streamingLabel="working" />
        <AiSdkAssistantGroups
          messageId={message.id}
          groups={groups}
          streaming={streaming}
          lastTextPartIdx={lastTextIdx}
          onApproval={onApproval}
          onOpenPath={(path) => {
            void openWorkspaceFile(path);
          }}
          wrapPart={(node, key) => (
            <PartAppear key={key}>{node}</PartAppear>
          )}
          icons={{
            file: (
              <HugeiconsIcon icon={File01Icon} size={13} strokeWidth={1.75} />
            ),
            web: (
              <HugeiconsIcon
                icon={GlobalSearchIcon}
                size={13}
                strokeWidth={1.75}
              />
            ),
            terminal: (
              <HugeiconsIcon icon={TerminalIcon} size={13} strokeWidth={1.75} />
            ),
          }}
          renderPart={({ part, streaming: partStreaming, onApproval: approve }) => (
            <RenderedPart
              part={part as AnyPart}
              onApproval={approve}
              streaming={partStreaming}
            />
          )}
        />
      </MessageContent>
      {showRunActions ? (
        <MessageActions className="opacity-0 transition-opacity focus-within:opacity-100 group-hover:opacity-100">
          {streaming ? (
            <HoverActionButton title="Stop generating" onClick={() => onStop?.()}>
              <HugeiconsIcon icon={Cancel01Icon} size={11} strokeWidth={1.75} />
              Stop
            </HoverActionButton>
          ) : (
            <HoverActionButton title="Retry" onClick={() => onRetry?.()}>
              <HugeiconsIcon icon={Refresh01Icon} size={11} strokeWidth={1.75} />
              Retry
            </HoverActionButton>
          )}
        </MessageActions>
      ) : null}
    </Message>
  );
});

const PartAppear = memo(function PartAppear({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <motion.div
      initial={{ opacity: 0, y: 6 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.22, ease: [0.22, 1, 0.36, 1] }}
      style={{ willChange: "transform, opacity" }}
    >
      {children}
    </motion.div>
  );
});

const RenderedPart = memo(function RenderedPart({
  part,
  onApproval,
  streaming,
}: {
  part: AnyPart;
  onApproval: (id: string, approved: boolean) => void;
  streaming: boolean;
}) {
  if (part.type === "text") {
    return (
      <MessageResponse streaming={streaming}>
        {(part as unknown as { text: string }).text}
      </MessageResponse>
    );
  }

  if (part.type === "reasoning") {
    return (
      <Reasoning>
        <ReasoningTrigger />
        <ReasoningContent>
          {(part as unknown as { text: string }).text}
        </ReasoningContent>
      </Reasoning>
    );
  }

  if (
    part.type === "dynamic-tool" ||
    (typeof part.type === "string" && part.type.startsWith("tool-"))
  ) {
    return (
      <RenderedTool
        part={part as unknown as AnyToolPart}
        onApproval={onApproval}
      />
    );
  }

  return null;
});

const RenderedTool = memo(function RenderedTool({
  part,
  onApproval,
}: {
  part: AnyToolPart;
  onApproval: (id: string, approved: boolean) => void;
}) {
  const assertiveAnnounce = usePreferencesStore(
    (s) => s.approvalAnnounceAssertive,
  );
  const toolName =
    part.type === "dynamic-tool"
      ? part.toolName
      : part.type.replace(/^tool-/, "");

  if (part.state === "approval-requested") {
    return (
      <AiToolApproval
        part={{
          state: "approval-requested",
          approval: { id: part.approval.id },
          input: part.input,
        }}
        toolName={toolName}
        assertiveAnnounce={assertiveAnnounce}
        onRespond={(approved) => onApproval(part.approval.id, approved)}
      />
    );
  }

  return (
    <Tool
      toolName={toolName}
      state={part.state}
      input={part.input}
      output={"output" in part ? part.output : undefined}
      errorText={"errorText" in part ? part.errorText : undefined}
      defaultOpen={toolName === "list_directory"}
    />
  );
});
