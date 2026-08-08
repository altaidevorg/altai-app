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
  AiChatViewFrame,
  AiSdkAssistantGroups,
  AiSdkToolPartSwitch,
  AiSdkUiPartSwitch,
  AiToolApproval,
  AssistantBrandLabel,
  AiUserTurnBody,
  buildTranscriptPartGroups,
  HoverActionButton,
  indexOfLastTextPart,
  joinMessageTextParts,
  prepareUserTurnDisplay,
  resolveAssistantRunActionMode,
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

  const onApproval = useCallback(
    (id: string, approved: boolean) => addToolApprovalResponse({ id, approved }),
    [addToolApprovalResponse],
  );

  const statusPill = <AgentStatusPill hideError />;

  return (
    <AiChatViewFrame
      messages={messages}
      status={status}
      announce={chatAnnounce}
      retryableFailure={retryableFailure}
      error={error ?? null}
      onDismissError={clearError}
      emptyStatus={statusPill}
      endStatus={statusPill}
      renderRoot={({ "aria-live": ariaLive, isEmpty, body }) => (
        <Conversation
          className="altai-ai-conversation overflow-x-hidden"
          aria-live={ariaLive}
        >
          <ConversationContent className="min-w-0">{body}</ConversationContent>
          {!isEmpty ? <ConversationScrollButton /> : null}
        </Conversation>
      )}
      renderMessage={({ message, streaming, canRetry }) => (
        <RenderedMessage
          key={message.id}
          message={message}
          onApproval={onApproval}
          streaming={streaming}
          canRetry={canRetry}
          onRetry={() => void retryFailedRun()}
          onStop={() => void stop?.()}
        />
      )}
    />
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

  const runActionMode = resolveAssistantRunActionMode({
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
      {runActionMode !== "hidden" ? (
        <MessageActions className="opacity-0 transition-opacity focus-within:opacity-100 group-hover:opacity-100">
          {runActionMode === "stop" ? (
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
  const viewPart = part as { type?: string; text?: string };
  return (
    <AiSdkUiPartSwitch
      part={viewPart}
      streaming={streaming}
      renderText={(text, partStreaming) => (
        <MessageResponse streaming={partStreaming}>{text}</MessageResponse>
      )}
      renderReasoning={(text) => (
        <Reasoning>
          <ReasoningTrigger />
          <ReasoningContent>{text}</ReasoningContent>
        </Reasoning>
      )}
      renderTool={() => (
        <RenderedTool
          part={part as unknown as AnyToolPart}
          onApproval={onApproval}
        />
      )}
    />
  );
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
  const like = part as {
    type?: string;
    toolName?: string;
    state?: string;
    input?: unknown;
    approval?: { id?: string };
    output?: unknown;
    errorText?: string;
  };
  return (
    <AiSdkToolPartSwitch
      part={like}
      renderApproval={(approval) => (
        <AiToolApproval
          part={{
            state: "approval-requested",
            approval: { id: approval.approvalId },
            input: approval.input,
          }}
          toolName={approval.toolName}
          assertiveAnnounce={assertiveAnnounce}
          onRespond={(approved) => onApproval(approval.approvalId, approved)}
        />
      )}
      renderCard={(card) => (
        <Tool
          toolName={card.toolName}
          state={card.state as AnyToolPart["state"]}
          input={card.input}
          output={card.output}
          errorText={card.errorText}
          defaultOpen={card.defaultOpen}
        />
      )}
    />
  );
});
