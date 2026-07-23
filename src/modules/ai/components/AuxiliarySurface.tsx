import { cn } from "@/lib/utils";
import { Cancel01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import type { ReactNode } from "react";

const CLOSE_BTN =
  "inline-flex size-7 shrink-0 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-foreground/[0.06] hover:text-foreground";

const HEADER =
  "flex shrink-0 items-center gap-2 border-b border-border/50 px-3 py-2.5";

/**
 * Shared header for AI auxiliary surfaces (inbox, tasks, automations,
 * change review, inspector). Keeps title scale, padding, and close control
 * identical across overlays.
 */
export function SurfaceHeader({
  title,
  subtitle,
  onClose,
  actions,
  className,
}: {
  title: string;
  subtitle?: ReactNode;
  onClose?: () => void;
  actions?: ReactNode;
  className?: string;
}) {
  return (
    <header className={cn(HEADER, className)}>
      <div className="min-w-0 flex-1">
        <h2 className="text-[12px] font-semibold text-foreground">{title}</h2>
        {subtitle ? (
          <div className="mt-0.5 text-[10px] text-muted-foreground">{subtitle}</div>
        ) : null}
      </div>
      {actions}
      {onClose ? (
        <button
          type="button"
          onClick={onClose}
          aria-label={`Close ${title}`}
          className={CLOSE_BTN}
        >
          <HugeiconsIcon icon={Cancel01Icon} size={13} strokeWidth={1.75} />
        </button>
      ) : null}
    </header>
  );
}

/**
 * Full-bleed overlay shell used by inbox / tasks / automations / review.
 * Solid background (no mismatched blur opacities) so every surface matches.
 */
export function AuxiliarySurface({
  title,
  subtitle,
  onClose,
  actions,
  children,
  className,
  bodyClassName,
}: {
  title: string;
  subtitle?: ReactNode;
  onClose?: () => void;
  actions?: ReactNode;
  children: ReactNode;
  className?: string;
  bodyClassName?: string;
}) {
  return (
    <section
      aria-label={title}
      className={cn(
        "absolute inset-0 z-30 flex flex-col bg-background",
        className,
      )}
    >
      <SurfaceHeader
        title={title}
        subtitle={subtitle}
        onClose={onClose}
        actions={actions}
      />
      <div className={cn("flex min-h-0 flex-1 flex-col overflow-hidden", bodyClassName)}>
        {children}
      </div>
    </section>
  );
}

/** Icon-sized secondary action (refresh, etc.) that matches the close control. */
export function SurfaceIconAction({
  label,
  onClick,
  disabled,
  children,
}: {
  label: string;
  onClick: () => void;
  disabled?: boolean;
  children: ReactNode;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      aria-label={label}
      className={cn(CLOSE_BTN, "disabled:opacity-45")}
    >
      {children}
    </button>
  );
}
