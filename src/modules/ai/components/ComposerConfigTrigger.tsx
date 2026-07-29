import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { ArrowDown01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import type { ComponentProps, ReactNode } from "react";

type ComposerConfigTriggerProps = Omit<
  ComponentProps<typeof Button>,
  "children" | "size" | "variant"
> & {
  icon: ReactNode;
  label: ReactNode;
};

/**
 * The shared composer control for choices that configure the next run.
 * It intentionally only owns the trigger: agent and model menus have very
 * different information densities, but their selected state must read alike.
 */
export function ComposerConfigTrigger({
  icon,
  label,
  className,
  ...props
}: ComposerConfigTriggerProps) {
  return (
    <Button
      type="button"
      variant="ghost"
      size="sm"
      className={cn(
        "altai-ai-composer-config-trigger group flex h-7 min-w-0 max-w-[11rem] items-center rounded-md px-2 text-[11.5px] text-foreground/85 transition-colors hover:bg-foreground/[0.055]",
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
    </Button>
  );
}
