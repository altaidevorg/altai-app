import { ArrowDown01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import {
  forwardRef,
  type ButtonHTMLAttributes,
  type ReactNode,
} from "react";
import { cn } from "../lib/cn.js";

export type ComposerConfigTriggerProps = Omit<
  ButtonHTMLAttributes<HTMLButtonElement>,
  "children"
> & {
  icon: ReactNode;
  label: ReactNode;
};

/**
 * Shared composer control for choices that configure the next run.
 * Owns only the trigger chrome; agent/model menus stay host-specific.
 */
export const ComposerConfigTrigger = forwardRef<
  HTMLButtonElement,
  ComposerConfigTriggerProps
>(function ComposerConfigTrigger(
  { icon, label, className, type = "button", ...props },
  ref,
) {
  return (
    <button
      ref={ref}
      type={type}
      className={cn(
        "altai-ai-composer-config-trigger group inline-flex h-7 min-w-0 max-w-[11rem] items-center gap-1 rounded-md border border-transparent px-2 text-[11.5px] text-foreground/85 transition-colors outline-none select-none",
        "hover:bg-foreground/[0.055] focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/30",
        "disabled:pointer-events-none disabled:opacity-50",
        className,
      )}
      {...props}
    >
      <span className="altai-ai-composer-config-trigger-label">
        {icon}
        <span className="min-w-0 truncate font-medium">{label}</span>
      </span>
      <HugeiconsIcon
        icon={ArrowDown01Icon}
        size={11}
        strokeWidth={2}
        className="shrink-0 opacity-60 transition-opacity group-hover:opacity-90"
      />
    </button>
  );
});
