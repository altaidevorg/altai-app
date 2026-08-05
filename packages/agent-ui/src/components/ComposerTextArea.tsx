import {
  forwardRef,
  type TextareaHTMLAttributes,
} from "react";
import { cn } from "../lib/cn.js";

export type ComposerTextAreaProps = TextareaHTMLAttributes<HTMLTextAreaElement>;

/**
 * Shared composer text-entry chrome. Hosts own value state, keyboard routing,
 * picker detection, autoresize behavior, and submission.
 */
export const ComposerTextArea = forwardRef<
  HTMLTextAreaElement,
  ComposerTextAreaProps
>(function ComposerTextArea(
  {
    className,
    rows = 2,
    "aria-label": ariaLabel = "Message ALTAI",
    ...props
  },
  ref,
) {
  return (
    <textarea
      ref={ref}
      rows={rows}
      aria-label={ariaLabel}
      className={cn(
        "altai-ai-composer-textarea block max-h-44 min-h-[48px] w-full min-w-0 max-w-full resize-none bg-transparent pr-1 text-[13px] leading-5 text-foreground outline-none placeholder:text-muted-foreground/55",
        className,
      )}
      {...props}
    />
  );
});
