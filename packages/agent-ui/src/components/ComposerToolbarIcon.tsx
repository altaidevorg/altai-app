import type { ReactElement, ReactNode } from "react";
import { cn } from "../lib/cn.js";

export type ComposerToolbarIconProps = {
  title: string;
  onClick?: () => void;
  disabled?: boolean;
  className?: string;
  children: ReactNode;
  /**
   * Host wraps the control (Desktop uses Radix Tooltip). Defaults to the bare
   * button.
   */
  renderTooltip?: (label: string, children: ReactElement) => ReactNode;
};

function defaultTooltip(_label: string, children: ReactElement): ReactNode {
  return children;
}

/**
 * Compact ghost icon button for the AI composer toolbar. Presentational only;
 * tooltip chrome stays on the host via `renderTooltip`.
 */
export function ComposerToolbarIcon({
  title,
  onClick,
  disabled,
  className,
  children,
  renderTooltip = defaultTooltip,
}: ComposerToolbarIconProps) {
  const button = (
    <button
      type="button"
      aria-label={title}
      title={title}
      onClick={onClick}
      disabled={disabled}
      className={cn(
        "inline-flex size-6 shrink-0 items-center justify-center rounded-md text-muted-foreground hover:bg-accent hover:text-foreground disabled:opacity-40",
        className,
      )}
    >
      {children}
    </button>
  );

  return <>{renderTooltip(title, button)}</>;
}
