/**
 * Ports-first flat transcript list for display messages (A6.38).
 * Groups consecutive tool rows; hosts render each message body.
 * No stores, Tauri, or HostPorts.
 */

import { useMemo, type ReactNode } from "react";
import { TranscriptToolGroup } from "./TranscriptToolGroup.js";
import { resolveChatAriaLive } from "../lib/chatTranscriptChrome.js";
import {
  buildDisplayTranscriptBlocks,
  type DisplayToolGroupKind,
  type TranscriptDisplayMessage,
} from "../lib/displayTranscriptBlocks.js";
import { cn } from "../lib/cn.js";

export type AiDisplayTranscriptListProps<
  T extends TranscriptDisplayMessage = TranscriptDisplayMessage,
> = {
  messages: readonly T[];
  /** Host announce preference (`off` | `polite` | `assertive`). */
  announce?: string;
  /** Element id (default matches VS Code active chat region). */
  id?: string;
  className?: string;
  /**
   * Render one display bubble (user / assistant / tool / meta).
   * Called for ungrouped messages and children of tool groups.
   */
  renderMessage: (message: T) => ReactNode;
  /**
   * Optional icon for collapsed tool groups. When omitted, groups render
   * without a custom icon.
   */
  renderGroupIcon?: (kind: DisplayToolGroupKind) => ReactNode;
};

/**
 * role=log transcript that collapses tool bursts via
 * `buildDisplayTranscriptBlocks`. Hosts inject bubble chrome via
 * `renderMessage` (edit / open / copy stay host-owned).
 */
export function AiDisplayTranscriptList<
  T extends TranscriptDisplayMessage = TranscriptDisplayMessage,
>({
  messages,
  announce = "polite",
  id = "altai-active-chat",
  className,
  renderMessage,
  renderGroupIcon,
}: AiDisplayTranscriptListProps<T>) {
  const ariaLive = resolveChatAriaLive(announce);
  const blocks = useMemo(
    () => buildDisplayTranscriptBlocks(messages),
    [messages],
  );

  return (
    <div
      className={cn("altai-chat-log", className)}
      role="log"
      aria-live={ariaLive}
      aria-relevant="additions"
      id={id}
    >
      {blocks.map((block) => {
        if (block.kind === "message") {
          return renderMessage(block.message);
        }
        const kind = block.groupKind;
        return (
          <div
            key={block.key}
            className="altai-chat-tool-group"
            data-kind={kind}
          >
            <TranscriptToolGroup
              label={block.label}
              countLabel={block.countLabel}
              preview={block.preview}
              icon={renderGroupIcon?.(kind) ?? null}
              previewMono={kind === "reads" || kind === "cmd"}
            >
              {block.messages.map((message) => renderMessage(message))}
            </TranscriptToolGroup>
          </div>
        );
      })}
    </div>
  );
}
