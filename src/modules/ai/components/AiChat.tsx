import {
  Conversation,
  ConversationContent,
  ConversationEmptyState,
  ConversationScrollButton,
} from "@/components/ai-elements/conversation";
import {
  Reasoning,
  ReasoningContent,
  ReasoningTrigger,
} from "@/components/ai-elements/reasoning";
import { Tool } from "@/components/ai-elements/tool";
import { cn } from "@/lib/utils";
import { motion } from "motion/react";
import { HugeiconsIcon } from "@hugeicons/react";
import {
  Cancel01Icon,
  File01Icon,
  GlobalSearchIcon,
  Refresh01Icon,
  TerminalIcon,
} from "@hugeicons/core-free-icons";
import { ALTAI_CMD_RE, resolveSlashCommand } from "../lib/slashCommands";
import { usePreferencesStore } from "@/modules/settings/preferences";
import { retryFailedRun, useChatStore } from "../store/chatStore";
import { useAgentRunsStore } from "../store/agentRunsStore";
import {
  isRecoverableAttentionMessage,
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
  AiToolApproval,
  AssistantBrandLabel,
  buildTranscriptPartGroups,
  cmdSummaryForToolPart,
  CommandSnippet,
  ContextChips,
  formatGroupPreview,
  HoverActionButton,
  pathBasename,
  stripUserContextBlocks,
  toolNameOf,
  transcriptPartKey,
  TranscriptReadPaths,
  TranscriptReadRow,
  TranscriptToolGroup,
  uniqueReadPaths,
  uniqueSummaries,
  webSummaryForToolPart,
  type ToolLikePart,
} from "@altai/agent-ui";
import { AgentStatusPill } from "./AgentStatusPill";
import { openWorkspaceFile } from "../lib/openChatHref";
import {
  Message,
  MessageActions,
  MessageContent,
  MessageResponse,
} from "@/components/ai-elements/message";

function ResolvedCommandSnippet({ name }: { name: string }) {
  const meta = resolveSlashCommand(name);
  return (
    <CommandSnippet
      name={name}
      meta={
        meta
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
          : null
      }
    />
  );
}

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
  const ariaLiveProp: "off" | "polite" | "assertive" =
    chatAnnounce === "off"
      ? "off"
      : chatAnnounce === "assertive"
        ? "assertive"
        : "polite";
  const lastMessage = messages[messages.length - 1];
  const streamingMessageId =
    status === "streaming" && lastMessage?.role === "assistant"
      ? lastMessage.id
      : null;

  const onApproval = useCallback(
    (id: string, approved: boolean) => addToolApprovalResponse({ id, approved }),
    [addToolApprovalResponse],
  );

  if (messages.length === 0) {
    return (
      <Conversation className="altai-ai-conversation overflow-x-hidden" aria-live={ariaLiveProp}>
        <ConversationContent className="min-w-0">
          <ConversationEmptyState
            title="Ask ALTAI anything"
            description="Explain command output, fix errors, generate snippets, or run a task."
          />
          {/* Live status stays inside the transcript viewport, not above the composer. */}
          <AgentStatusPill hideError />
        </ConversationContent>
      </Conversation>
    );
  }

  return (
    <Conversation className="altai-ai-conversation overflow-x-hidden" aria-live={ariaLiveProp}>
      <ConversationContent className="altai-ai-transcript mx-auto min-w-0 w-full max-w-[52rem] gap-5 px-4 py-5 @[44rem]:px-6">
        {messages.map((m, i) => (
          <RenderedMessage
            key={m.id}
            message={m}
            onApproval={onApproval}
            streaming={m.id === streamingMessageId}
            canRetry={
              retryableFailure &&
              m.role === "assistant" &&
              i === messages.length - 1 &&
              status !== "streaming"
            }
            onRetry={() => void retryFailedRun()}
            onStop={() => void stop?.()}
          />
        ))}
        {/* Agent working indicator — end of transcript, inside the chat scroll. */}
        <AgentStatusPill hideError />
        {error && (
          // role="alert" => assertive live region. Without this the chat
          // failure was silent to screen readers and the agent appeared
          // to hang. JAWS/NVDA/VoiceOver will interrupt and announce the
          // error message + "Dismiss" affordance.
          <div
            role="alert"
            aria-atomic="true"
            className={cn(
              "rounded-md border px-3 py-2 text-xs",
              isRecoverableAttentionMessage(error.message)
                ? "border-warning/40 bg-warning/10 text-foreground"
                : "border-destructive/40 bg-destructive/10 text-destructive",
            )}
          >
            <div className="font-medium">
              {isRecoverableAttentionMessage(error.message)
                ? "Run needs attention"
                : "Something went wrong."}
            </div>
            <div className="mt-0.5 leading-relaxed opacity-90">
              {error.message}
            </div>
            <button
              type="button"
              onClick={clearError}
              className="mt-1 underline opacity-80 hover:opacity-100"
            >
              Dismiss
            </button>
          </div>
        )}
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
  let lastTextIdx = -1;
  for (let i = message.parts.length - 1; i >= 0; i -= 1) {
    if (message.parts[i]?.type === "text") {
      lastTextIdx = i;
      break;
    }
  }
  if (message.role === "user") {
    const rawText = message.parts
      .filter((p): p is { type: "text"; text: string } => p.type === "text")
      .map((p) => p.text)
      .join("\n");

    const cmdMatch = rawText.match(ALTAI_CMD_RE);
    const commandName = cmdMatch?.[1] ?? null;
    const withoutCmd = cmdMatch ? rawText.slice(cmdMatch[0].length) : rawText;
    const stripped = stripUserContextBlocks(withoutCmd);

    return (
      <Message from="user" className="altai-ai-message">
        <MessageContent>
          {commandName ? <ResolvedCommandSnippet name={commandName} /> : null}
          {stripped.chips.length > 0 ? (
            <ContextChips chips={stripped.chips} />
          ) : null}
          {stripped.text ? (
            <p className="whitespace-pre-wrap break-words">{stripped.text}</p>
          ) : null}
        </MessageContent>
      </Message>
    );
  }

  const groups = useMemo(
    () => buildTranscriptPartGroups(message.parts as AnyPart[]),
    [message.parts],
  );

  const showRunActions = streaming || Boolean(canRetry);

  return (
    <Message from={message.role} className="altai-ai-message">
      <MessageContent>
        <AssistantBrandLabel streaming={streaming} streamingLabel="working" />
        <div className="flex min-w-0 flex-col gap-3">
          {groups.map((g) => {
            if (g.kind === "reads") {
              return (
                <PartAppear key={`${message.id}-${g.key}`}>
                  <ReadGroup parts={g.parts} />
                </PartAppear>
              );
            }
            if (g.kind === "web") {
              return (
                <PartAppear key={`${message.id}-${g.key}`}>
                  <WebGroup parts={g.parts} onApproval={onApproval} />
                </PartAppear>
              );
            }
            if (g.kind === "cmd") {
              return (
                <PartAppear key={`${message.id}-${g.key}`}>
                  <CommandGroup parts={g.parts} onApproval={onApproval} />
                </PartAppear>
              );
            }
            // g.kind === "single"
            const part = g.part;
            const isReadSingle =
              toolNameOf(part as ToolLikePart) === "read_file" &&
              ((part as { state?: string }).state ?? "") !==
                "approval-requested";
            if (isReadSingle) {
              return (
                <PartAppear key={`${message.id}-${g.key}`}>
                  <TranscriptReadRow part={part as ToolLikePart} />
                </PartAppear>
              );
            }
            return (
              <PartAppear key={`${message.id}-${g.key}`}>
                <RenderedPart
                  part={part}
                  onApproval={onApproval}
                  streaming={streaming && g.idx === lastTextIdx}
                />
              </PartAppear>
            );
          })}
        </div>
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

const ReadGroup = memo(function ReadGroup({ parts }: { parts: AnyPart[] }) {
  const paths = useMemo(
    () => uniqueReadPaths(parts as ToolLikePart[]),
    [parts],
  );
  const count = paths.length || parts.length;

  return (
    <TranscriptToolGroup
      label="Read"
      countLabel={`${count} file${count === 1 ? "" : "s"}`}
      preview={
        paths.length > 0
          ? formatGroupPreview(paths.map((p) => pathBasename(p)))
          : undefined
      }
      previewMono
      icon={
        <HugeiconsIcon icon={File01Icon} size={13} strokeWidth={1.75} />
      }
    >
      <TranscriptReadPaths
        paths={paths}
        onOpen={(path) => {
          void openWorkspaceFile(path);
        }}
      />
    </TranscriptToolGroup>
  );
});

const WebGroup = memo(function WebGroup({
  parts,
  onApproval,
}: {
  parts: AnyPart[];
  onApproval: (id: string, approved: boolean) => void;
}) {
  const summaries = useMemo(
    () => uniqueSummaries(parts as ToolLikePart[], webSummaryForToolPart),
    [parts],
  );
  const count = parts.length;
  const preview = formatGroupPreview(summaries);

  return (
    <TranscriptToolGroup
      label="Web"
      countLabel={`${count} call${count === 1 ? "" : "s"}`}
      preview={preview}
      icon={
        <HugeiconsIcon icon={GlobalSearchIcon} size={13} strokeWidth={1.75} />
      }
    >
      <div className="flex flex-col gap-1 px-2 py-1.5">
        {parts.map((p, i) => (
          <RenderedPart
            key={transcriptPartKey(p as ToolLikePart, i)}
            part={p}
            onApproval={onApproval}
            streaming={false}
          />
        ))}
      </div>
    </TranscriptToolGroup>
  );
});

const CommandGroup = memo(function CommandGroup({
  parts,
  onApproval,
}: {
  parts: AnyPart[];
  onApproval: (id: string, approved: boolean) => void;
}) {
  const summaries = useMemo(
    () => uniqueSummaries(parts as ToolLikePart[], cmdSummaryForToolPart),
    [parts],
  );
  const count = parts.length;
  const preview = formatGroupPreview(summaries, { separator: " · " });

  return (
    <TranscriptToolGroup
      label="Ran"
      countLabel={`${count} command${count === 1 ? "" : "s"}`}
      preview={preview}
      previewMono
      icon={
        <HugeiconsIcon icon={TerminalIcon} size={13} strokeWidth={1.75} />
      }
    >
      <div className="flex flex-col gap-1 px-2 py-1.5">
        {parts.map((p, i) => (
          <RenderedPart
            key={transcriptPartKey(p as ToolLikePart, i)}
            part={p}
            onApproval={onApproval}
            streaming={false}
          />
        ))}
      </div>
    </TranscriptToolGroup>
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
