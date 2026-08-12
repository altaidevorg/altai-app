import type { ReactNode } from "react";

export type ComposerPrimaryRowProps = {
  tools: ReactNode;
  permission?: ReactNode;
  submit: ReactNode;
};

/**
 * Primary composer toolbar chrome. Hosts inject file/context/voice controls,
 * permission UI, and send/stop behavior.
 */
export function ComposerPrimaryRow({
  tools,
  permission,
  submit,
}: ComposerPrimaryRowProps) {
  return (
    <div className="altai-ai-composer-primary flex w-full min-w-0 flex-wrap items-center gap-1 border-t border-border-subtle px-2 py-1.5 @[22rem]:px-2.5">
      <div className="altai-ai-composer-tools flex min-w-0 shrink-0 items-center gap-0.5 rounded-md bg-foreground/[0.035] p-0.5">
        {tools}
      </div>
      <div className="altai-ai-composer-actions ml-auto flex min-w-0 shrink-0 items-center gap-1">
        {permission ? (
          <div className="altai-ai-composer-permission-bottom flex shrink-0 items-center">
            {permission}
          </div>
        ) : null}
        <div className="altai-ai-composer-submit flex shrink-0 items-center">
          {submit}
        </div>
      </div>
    </div>
  );
}
