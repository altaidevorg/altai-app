import type { ReactNode } from "react";
import { cn } from "../lib/cn.js";

export type ComposerShellProps = {
  children: ReactNode;
  attachments?: ReactNode;
  busy?: boolean;
  className?: string;
};

/**
 * Shared composer card surface. Hosts inject attachment content and all
 * interactive rows while retaining picker, store, voice, and transport logic.
 */
export function ComposerShell({
  children,
  attachments,
  busy = false,
  className,
}: ComposerShellProps) {
  return (
    <div
      className={cn(
        "altai-ai-composer flex w-full min-w-0 max-w-full flex-col overflow-visible rounded-xl border border-border-subtle bg-transparent transition-[border-color,box-shadow] hover:border-border",
        busy && "opacity-95",
        className,
      )}
    >
      {attachments ? (
        <div className="altai-ai-composer-attachments border-b border-border-subtle px-2.5 py-2">
          {attachments}
        </div>
      ) : null}
      {children}
    </div>
  );
}
