/**
 * Ports-first flat display-message bubble shell (A6.51).
 * Hosts supply body / edit / action slots; package owns role chrome.
 */

import type { ReactNode } from "react";
import {
  chatDisplayBubbleClassName,
  chatDisplayRoleLabel,
} from "../lib/chatDisplayChrome.js";

export type AiDisplayMessageBubbleProps = {
  messageId: string;
  role: string;
  streaming?: boolean;
  /** When true, replace body with editSlot. */
  isEditing?: boolean;
  editSlot?: ReactNode;
  body?: ReactNode;
  /** Footer actions (hidden while editing). */
  actions?: ReactNode;
  className?: string;
  /**
   * Override the header label. Default uses chatDisplayRoleLabel(role);
   * pass null to hide.
   */
  label?: string | null;
};

/** Stable DOM id for message anchors / deep-links. */
export function displayMessageElementId(messageId: string): string {
  return `altai-msg-${messageId}`;
}

/**
 * Article bubble: role header + body/edit + optional action footer.
 * CSS classes match VS Code `altai-chat-bubble-*` host stylesheets.
 */
export function AiDisplayMessageBubble({
  messageId,
  role,
  streaming = false,
  isEditing = false,
  editSlot,
  body,
  actions,
  className,
  label,
}: AiDisplayMessageBubbleProps) {
  const resolvedLabel =
    label === undefined ? chatDisplayRoleLabel(role) : label;
  const bubbleClass = className
    ? `${chatDisplayBubbleClassName(role)} ${className}`
    : chatDisplayBubbleClassName(role);

  return (
    <article
      id={displayMessageElementId(messageId)}
      className={bubbleClass}
      data-role={role}
      data-streaming={streaming ? "true" : undefined}
    >
      {resolvedLabel ? (
        <header className="altai-chat-bubble-label">{resolvedLabel}</header>
      ) : null}
      {isEditing ? editSlot : body}
      {actions && !isEditing ? (
        <footer className="altai-chat-bubble-actions">{actions}</footer>
      ) : null}
    </article>
  );
}
