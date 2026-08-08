/**
 * Pure display-message action visibility (A6.39).
 * Hosts supply capabilities; UI only decides which hover actions to show.
 */

export type DisplayMessageActionFlags = {
  showEdit: boolean;
  showRetry: boolean;
  showCopy: boolean;
  showOpenFile: boolean;
  showOpenDiff: boolean;
};

export type DisplayMessageActionInput = {
  id: string;
  role: string;
  content: string;
  streaming?: boolean;
  fileUri?: string;
  filePath?: string;
  diffOriginalText?: string;
  diffModifiedText?: string;
};

/** When to offer a Copy action on a transcript bubble. */
export function canCopyDisplayMessage(input: {
  role: string;
  content: string;
  streaming?: boolean;
}): boolean {
  if (input.streaming) {
    return false;
  }
  if (input.role !== "user" && input.role !== "assistant") {
    return false;
  }
  return input.content.trim().length > 0;
}

/** Last finished/live assistant message id for retry attach. */
export function lastAssistantMessageId(
  messages: readonly { id: string; role: string }[],
): string | null {
  for (let i = messages.length - 1; i >= 0; i -= 1) {
    const message = messages[i];
    if (message?.role === "assistant") {
      return message.id;
    }
  }
  return null;
}

/**
 * Derive bubble hover-action visibility for flat ChatDisplayMessage hosts.
 * Capability gates and handlers come from the host; no HostPorts I/O here.
 */
export function resolveDisplayMessageActions(input: {
  message: DisplayMessageActionInput;
  lastAssistantId: string | null | undefined;
  canEditUserMessages: boolean;
  canRetry: boolean;
  canOpenFile: boolean;
  canOpenDiff: boolean;
  hasEditHandler: boolean;
  hasRetryHandler: boolean;
}): DisplayMessageActionFlags {
  const { message } = input;
  const streaming = Boolean(message.streaming);
  return {
    showEdit:
      message.role === "user" &&
      input.canEditUserMessages &&
      input.hasEditHandler &&
      !streaming,
    showRetry:
      message.role === "assistant" &&
      message.id === input.lastAssistantId &&
      input.canRetry &&
      input.hasRetryHandler &&
      !streaming,
    showCopy: canCopyDisplayMessage(message),
    showOpenFile:
      message.role === "tool" &&
      input.canOpenFile &&
      Boolean(message.fileUri) &&
      !streaming,
    showOpenDiff:
      message.role === "tool" &&
      input.canOpenDiff &&
      message.diffOriginalText !== undefined &&
      message.diffModifiedText !== undefined &&
      !streaming,
  };
}

/** True when the action footer should mount. */
export function hasDisplayMessageActions(
  flags: DisplayMessageActionFlags,
): boolean {
  return (
    flags.showEdit ||
    flags.showRetry ||
    flags.showCopy ||
    flags.showOpenFile ||
    flags.showOpenDiff
  );
}
