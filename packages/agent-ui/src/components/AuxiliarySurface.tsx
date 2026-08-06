import { Cancel01Icon, Search01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon, type IconSvgElement } from "@hugeicons/react";
import type { ReactNode } from "react";
import { cn } from "../lib/cn.js";

const CLOSE_BTN =
  "inline-flex size-7 shrink-0 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-foreground/[0.06] hover:text-foreground";

const HEADER =
  "flex min-h-11 shrink-0 items-center gap-2 border-b border-border-subtle px-3 py-2";

/**
 * Shared header for AI auxiliary surfaces (inbox, tasks, automations,
 * change review, inspector). Keeps title scale, padding, and close control
 * identical across overlays.
 */
export function SurfaceHeader({
  title,
  subtitle,
  eyebrow,
  icon,
  status,
  onClose,
  actions,
  className,
}: {
  title: string;
  subtitle?: ReactNode;
  eyebrow?: ReactNode;
  icon?: IconSvgElement;
  status?: ReactNode;
  onClose?: () => void;
  actions?: ReactNode;
  className?: string;
}) {
  return (
    <header className={cn(HEADER, "min-h-[58px] px-3.5 py-2.5", className)}>
      {icon ? (
        <span className="inline-flex size-8 shrink-0 items-center justify-center rounded-lg border border-border bg-muted text-muted-foreground">
          <HugeiconsIcon icon={icon} size={15} strokeWidth={1.75} />
        </span>
      ) : null}
      <div className="min-w-0 flex-1">
        {eyebrow ? (
          <div className="mb-0.5 text-[8.5px] font-semibold uppercase tracking-[0.14em] text-muted-foreground/70">
            {eyebrow}
          </div>
        ) : null}
        <div className="flex min-w-0 items-center gap-2">
          <h2 className="truncate text-[12.5px] font-semibold text-foreground">
            {title}
          </h2>
          {status}
        </div>
        {subtitle ? (
          <div className="mt-0.5 truncate text-[9.5px] text-muted-foreground">
            {subtitle}
          </div>
        ) : null}
      </div>
      {actions ? <div className="flex shrink-0 items-center gap-1">{actions}</div> : null}
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
 * Same card surface as the workspace sidebar so menus share one chrome.
 */
export function AuxiliarySurface({
  title,
  subtitle,
  eyebrow,
  icon,
  status,
  onClose,
  actions,
  navigation,
  children,
  className,
  bodyClassName,
  presentation = "overlay",
}: {
  title: string;
  subtitle?: ReactNode;
  eyebrow?: ReactNode;
  icon?: IconSvgElement;
  status?: ReactNode;
  onClose?: () => void;
  actions?: ReactNode;
  navigation?: ReactNode;
  children: ReactNode;
  className?: string;
  bodyClassName?: string;
  /**
   * `overlay` — absolute full-bleed panel (AI side drawer).
   * `embedded` — in-flow fill for host shells like Operations.
   */
  presentation?: "overlay" | "embedded";
}) {
  return (
    <section
      aria-label={title}
      className={cn(
        presentation === "overlay"
          ? "absolute inset-0 z-30 flex flex-col bg-card"
          : "flex h-full min-h-0 flex-col overflow-hidden bg-card",
        className,
      )}
    >
      <SurfaceHeader
        title={title}
        subtitle={subtitle}
        eyebrow={eyebrow}
        icon={icon}
        status={status}
        onClose={onClose}
        actions={actions}
      />
      {navigation}
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

export function SurfaceSearch({
  value,
  onChange,
  placeholder,
  label = placeholder,
  className,
}: {
  value: string;
  onChange: (value: string) => void;
  placeholder: string;
  label?: string;
  className?: string;
}) {
  return (
    <label
      className={cn(
        "flex h-7 min-w-0 items-center gap-2 rounded-md border border-border bg-muted/45 px-2 text-muted-foreground transition-colors focus-within:border-ring focus-within:bg-muted",
        className,
      )}
    >
      <HugeiconsIcon
        icon={Search01Icon}
        size={12}
        strokeWidth={1.8}
        className="shrink-0"
      />
      <span className="sr-only">{label}</span>
      <input
        value={value}
        onChange={(event) => onChange(event.target.value)}
        placeholder={placeholder}
        aria-label={label}
        className="min-w-0 flex-1 bg-transparent text-[10.5px] text-foreground outline-none placeholder:text-muted-foreground/65"
      />
      {value ? (
        <button
          type="button"
          onClick={() => onChange("")}
          aria-label="Clear search"
          className="-mr-1 inline-flex size-5 items-center justify-center rounded text-muted-foreground hover:bg-accent hover:text-foreground"
        >
          <HugeiconsIcon icon={Cancel01Icon} size={10} strokeWidth={2} />
        </button>
      ) : null}
    </label>
  );
}

export function SurfaceTabs({
  items,
  value,
  onChange,
  label,
  className,
}: {
  items: Array<{ id: string; label: string; count?: number }>;
  value: string;
  onChange: (value: string) => void;
  label: string;
  className?: string;
}) {
  return (
    <div
      role="tablist"
      aria-label={label}
      className={cn(
        "flex min-w-0 items-center gap-0.5 overflow-x-auto rounded-md border border-border bg-muted/35 p-0.5",
        className,
      )}
    >
      {items.map((item) => {
        const active = item.id === value;
        return (
          <button
            key={item.id}
            type="button"
            role="tab"
            aria-selected={active}
            onClick={() => onChange(item.id)}
            className={cn(
              "flex h-6 shrink-0 items-center gap-1 rounded px-2 text-[9.5px] font-medium transition-colors",
              active
                ? "bg-card text-foreground shadow-sm"
                : "text-muted-foreground hover:bg-accent hover:text-foreground",
            )}
          >
            {item.label}
            {typeof item.count === "number" ? (
              <span
                className={cn(
                  "min-w-4 rounded px-1 text-center text-[8.5px] tabular-nums",
                  active ? "bg-foreground/[0.08]" : "bg-foreground/[0.05]",
                )}
              >
                {item.count}
              </span>
            ) : null}
          </button>
        );
      })}
    </div>
  );
}

export function SurfaceSectionHeader({
  title,
  description,
  count,
  action,
  className,
}: {
  title: string;
  description?: ReactNode;
  count?: number;
  action?: ReactNode;
  className?: string;
}) {
  return (
    <div className={cn("flex min-w-0 items-end gap-2", className)}>
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-1.5">
          <h3 className="text-[10.5px] font-semibold text-foreground">{title}</h3>
          {typeof count === "number" ? (
            <span className="rounded bg-foreground/[0.06] px-1.5 py-0.5 text-[8.5px] font-medium tabular-nums text-muted-foreground">
              {count}
            </span>
          ) : null}
        </div>
        {description ? (
          <div className="mt-0.5 text-[9px] leading-relaxed text-muted-foreground">
            {description}
          </div>
        ) : null}
      </div>
      {action}
    </div>
  );
}

export function SurfaceEmptyState({
  icon,
  title,
  description,
  action,
  className,
}: {
  icon?: IconSvgElement;
  title: string;
  description: ReactNode;
  action?: ReactNode;
  className?: string;
}) {
  return (
    <div
      className={cn(
        "flex min-h-44 flex-col items-center justify-center rounded-lg border border-dashed border-border bg-muted/15 px-5 py-8 text-center",
        className,
      )}
    >
      {icon ? (
        <span className="mb-3 inline-flex size-9 items-center justify-center rounded-lg border border-border bg-card text-muted-foreground">
          <HugeiconsIcon icon={icon} size={16} strokeWidth={1.75} />
        </span>
      ) : null}
      <h3 className="text-[11px] font-semibold text-foreground">{title}</h3>
      <p className="mt-1 max-w-64 text-[9.5px] leading-relaxed text-muted-foreground">
        {description}
      </p>
      {action ? <div className="mt-3">{action}</div> : null}
    </div>
  );
}
