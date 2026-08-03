import type { ReactNode } from "react";

export type CommandSnippetMeta = {
  invocation: string;
  label: string;
  /** Host-rendered icon (keeps slash-command registries out of the package). */
  icon: ReactNode;
};

export type CommandSnippetProps = {
  name: string;
  meta?: CommandSnippetMeta | null;
};

/**
 * Compact chip for a slash command attached to a user message.
 * Unknown commands fall back to a plain `/{name}` badge.
 */
export function CommandSnippet({ name, meta }: CommandSnippetProps) {
  if (!meta) {
    return (
      <div className="inline-flex items-center gap-1.5 rounded-md border border-border/50 bg-muted/40 px-2 py-1 font-mono text-[11px]">
        /{name}
      </div>
    );
  }
  return (
    <div className="inline-flex max-w-full items-center gap-2 rounded-md border border-border/50 bg-muted/40 px-2 py-1">
      {meta.icon}
      <span className="font-mono text-[11px] text-foreground">
        {meta.invocation}
      </span>
      <span className="truncate text-[11px] text-muted-foreground">
        {meta.label}
      </span>
    </div>
  );
}
