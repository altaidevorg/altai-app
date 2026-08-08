/**
 * Ports-first flat display-message body (path/url/code segments) (A6.57).
 * Hosts inject open callbacks + capability flags (no HostPorts import).
 */

import type { ReactNode } from "react";
import { ChatExternalLink, ChatPathLink } from "./ChatPathLink.js";
import {
  segmentChatContent,
  type ChatContentSegment,
} from "../lib/chatContentSegments.js";
import { cn } from "../lib/cn.js";

export type AiDisplayMessageContentProps = {
  content: string;
  streaming?: boolean;
  canOpenFile?: boolean;
  canOpenUrl?: boolean;
  busy?: boolean;
  onOpenPath?: (path: string) => void;
  onOpenUrl?: (href: string) => void;
  className?: string;
  /** Optional stream caret override (default ▍). */
  streamingMarker?: ReactNode;
};

/**
 * Segmented chat body: text, fenced code, path links, external links.
 * Classes match VS Code `altai-chat-*` host stylesheets.
 */
export function AiDisplayMessageContent({
  content,
  streaming = false,
  canOpenFile = false,
  canOpenUrl = false,
  busy = false,
  onOpenPath,
  onOpenUrl,
  className,
  streamingMarker = "▍",
}: AiDisplayMessageContentProps) {
  const segments = segmentChatContent(content);

  return (
    <div className={cn("altai-chat-bubble-body", className)}>
      {segments.map((segment, index) => (
        <Segment
          key={segmentKey(segment, index)}
          segment={segment}
          canOpenFile={canOpenFile}
          canOpenUrl={canOpenUrl}
          busy={busy}
          onOpenPath={onOpenPath}
          onOpenUrl={onOpenUrl}
        />
      ))}
      {streaming ? (
        <span className="altai-chat-streaming" aria-hidden="true">
          {streamingMarker}
        </span>
      ) : null}
    </div>
  );
}

function segmentKey(segment: ChatContentSegment, index: number): string {
  const detail =
    segment.kind === "text" || segment.kind === "code"
      ? segment.text.slice(0, 24)
      : segment.kind === "path"
        ? segment.path
        : segment.href;
  return `${segment.kind}:${index}:${detail}`;
}

function Segment({
  segment,
  canOpenFile,
  canOpenUrl,
  busy,
  onOpenPath,
  onOpenUrl,
}: {
  segment: ChatContentSegment;
  canOpenFile: boolean;
  canOpenUrl: boolean;
  busy: boolean;
  onOpenPath?: (path: string) => void;
  onOpenUrl?: (href: string) => void;
}) {
  if (segment.kind === "text") {
    return <>{segment.text}</>;
  }
  if (segment.kind === "code") {
    return (
      <pre className="altai-chat-code" data-lang={segment.lang ?? undefined}>
        <code>{segment.text}</code>
      </pre>
    );
  }
  if (segment.kind === "path") {
    if (!canOpenFile || !onOpenPath) {
      return (
        <span className="altai-chat-path is-static" title={segment.path}>
          {segment.text}
        </span>
      );
    }
    return (
      <ChatPathLink
        path={segment.path}
        onOpen={() => {
          if (!busy) {
            onOpenPath(segment.path);
          }
        }}
        className="altai-chat-path"
      >
        {segment.text}
      </ChatPathLink>
    );
  }
  if (!canOpenUrl || !onOpenUrl) {
    return (
      <span className="altai-chat-url is-static" title={segment.href}>
        {segment.text}
      </span>
    );
  }
  return (
    <ChatExternalLink
      href={segment.href}
      onOpen={() => {
        if (!busy) {
          onOpenUrl(segment.href);
        }
      }}
      className="altai-chat-url"
    >
      {segment.text}
    </ChatExternalLink>
  );
}
