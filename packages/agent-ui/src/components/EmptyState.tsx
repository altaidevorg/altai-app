import { SparklesIcon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";

export type EmptyStateProps = {
  agentName: string;
};

/**
 * Empty chat home shown when the active session has no messages. Purely
 * presentational; the host supplies the active agent name.
 */
export function EmptyState({ agentName }: EmptyStateProps) {
  return (
    <div className="altai-ai-task-home flex min-h-0 flex-1 flex-col overflow-y-auto px-4 py-5 @[36rem]:px-6 @[36rem]:py-7">
      <div className="mx-auto flex w-full max-w-[32rem] flex-1 flex-col justify-center">
        <div className="altai-ai-task-header">
          <div className="flex size-9 shrink-0 items-center justify-center rounded-xl border border-primary/20 bg-primary/10 text-primary">
            <HugeiconsIcon icon={SparklesIcon} size={17} strokeWidth={1.75} />
          </div>
          <div className="min-w-0">
            <div className="text-[10px] font-medium uppercase tracking-[0.13em] text-muted-foreground">
              {agentName} · ready
            </div>
            <h2 className="mt-1.5 text-[20px] font-semibold tracking-tight text-foreground">
              Start with the outcome
            </h2>
            <p className="mt-1 max-w-[31rem] text-[11.5px] leading-relaxed text-muted-foreground">
              Describe what should change and how we will know it is done. ALTAI
              can inspect context, work across files, and verify the result.
            </p>
          </div>
        </div>
      </div>

      <div className="flex shrink-0 items-center justify-center gap-1.5 pt-4 text-[10px] text-muted-foreground/70">
        <span>Files, terminal, and previews stay available from Open IDE.</span>
      </div>
    </div>
  );
}
