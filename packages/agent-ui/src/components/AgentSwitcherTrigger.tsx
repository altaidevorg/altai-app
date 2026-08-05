import { ArrowDown01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon, type IconSvgElement } from "@hugeicons/react";
import {
  forwardRef,
  type ButtonHTMLAttributes,
} from "react";
import { cn } from "../lib/cn.js";
import { ComposerConfigTrigger } from "./ComposerConfigTrigger.js";

export type AgentSwitcherTriggerVariant =
  | "default"
  | "mini"
  | "toolbar"
  | "toolbar-icon";

export type AgentSwitcherTriggerProps = Omit<
  ButtonHTMLAttributes<HTMLButtonElement>,
  "children"
> & {
  name: string;
  icon: IconSvgElement;
  variant?: AgentSwitcherTriggerVariant;
};

/**
 * Presentational trigger for the agent picker. The host owns menu chrome,
 * agent state, icon selection, and settings navigation.
 */
export const AgentSwitcherTrigger = forwardRef<
  HTMLButtonElement,
  AgentSwitcherTriggerProps
>(function AgentSwitcherTrigger(
  {
    name,
    icon,
    variant = "default",
    className,
    type = "button",
    ...props
  },
  ref,
) {
  const label = `Switch agent — current: ${name}`;
  const title = `Agent: ${name}`;

  if (variant === "toolbar") {
    return (
      <ComposerConfigTrigger
        ref={ref}
        type={type}
        icon={
          <HugeiconsIcon
            icon={icon}
            size={13}
            strokeWidth={1.75}
            className="shrink-0 opacity-80"
          />
        }
        label={name}
        className={cn("max-w-[9rem]", className)}
        aria-label={label}
        title={title}
        {...props}
      />
    );
  }

  const iconOnly = variant === "toolbar-icon";

  return (
    <button
      ref={ref}
      type={type}
      className={cn(
        "altai-agent-switcher-trigger group inline-flex shrink-0 items-center justify-center border border-transparent font-medium outline-none transition-colors select-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/30 disabled:pointer-events-none disabled:opacity-50",
        variant === "default"
          ? "h-6 gap-1 rounded-md border-border/60 bg-card px-1.5 text-[10.5px] text-muted-foreground hover:border-border hover:bg-foreground/[0.055]"
          : variant === "mini"
            ? "mr-1 h-6 gap-1 rounded-md px-2.5 text-xs text-muted-foreground hover:bg-muted hover:text-foreground"
            : "size-7 rounded-md p-0 text-muted-foreground hover:bg-foreground/[0.055]",
        className,
      )}
      aria-label={label}
      title={title}
      {...props}
    >
      <HugeiconsIcon
        icon={icon}
        size={11}
        strokeWidth={1.75}
        className="shrink-0"
      />
      {!iconOnly ? (
        <>
          <span className="max-w-[7rem] truncate">{name}</span>
          <HugeiconsIcon
            icon={ArrowDown01Icon}
            size={10}
            strokeWidth={2}
            className="shrink-0 opacity-70"
          />
        </>
      ) : null}
    </button>
  );
});
