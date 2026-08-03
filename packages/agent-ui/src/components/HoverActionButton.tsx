import type { ButtonHTMLAttributes, ReactNode } from "react";

export type HoverActionButtonProps = Omit<
  ButtonHTMLAttributes<HTMLButtonElement>,
  "title" | "onClick" | "children"
> & {
  /** Accessible label + tooltip text. */
  title: string;
  onClick: () => void;
  children?: ReactNode;
};

/**
 * Compact inline action button for chat message hover affordances
 * (e.g. Stop generating, Retry). Hosts own icons and behavior.
 */
export function HoverActionButton({
  title,
  onClick,
  children,
  ...props
}: HoverActionButtonProps) {
  return (
    <button
      type="button"
      title={title}
      aria-label={title}
      onClick={onClick}
      className="inline-flex items-center gap-1 rounded-md px-2 py-1 text-[10.5px] text-muted-foreground transition-colors hover:bg-foreground/10 hover:text-foreground"
      {...props}
    >
      {children}
    </button>
  );
}
