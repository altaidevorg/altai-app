import type { MouseEvent, ReactNode } from "react";
import { cn } from "../lib/cn.js";

export type ChatPathLinkProps = {
  path: string;
  /** Host opens the path in its editor (Desktop: openWorkspaceFile). */
  onOpen: (path: string) => void;
  className?: string;
  title?: string;
  children?: ReactNode;
};

/** Clickable workspace path that opens the file in the host editor. */
export function ChatPathLink({
  path,
  onOpen,
  className,
  title,
  children,
}: ChatPathLinkProps) {
  if (!path.trim()) return null;
  return (
    <button
      type="button"
      className={cn(
        "min-w-0 max-w-full truncate text-left hover:underline",
        "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
        className,
      )}
      title={title ?? path}
      onClick={(e) => {
        e.preventDefault();
        e.stopPropagation();
        onOpen(path);
      }}
    >
      {children ?? path}
    </button>
  );
}

export type ChatExternalLinkProps = {
  href: string;
  /** Host opens the URL (Desktop: openChatHref / Tauri opener). */
  onOpen: (href: string) => void;
  className?: string;
  children?: ReactNode;
};

/** External URL that opens via the host opener (not window.open). */
export function ChatExternalLink({
  href,
  onOpen,
  className,
  children,
}: ChatExternalLinkProps) {
  if (!href.trim()) return null;
  return (
    <a
      href={href}
      className={cn(
        "cursor-pointer hover:underline",
        "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
        className,
      )}
      onClick={(e: MouseEvent<HTMLAnchorElement>) => {
        e.preventDefault();
        e.stopPropagation();
        onOpen(href);
      }}
    >
      {children ?? href}
    </a>
  );
}
