/**
 * Altai shared UI layer.
 *
 * Reusable variants built on top of the shadcn primitives in
 * `src/components/ui`, expressed through the semantic design tokens in
 * globals.css (near-black canvas, lime primary, semantic status colors).
 *
 * Use these instead of sprinkling ad-hoc color classes per screen so the
 * IDE-wide visual language stays consistent. They are intentionally thin —
 * behaviour, handlers and accessibility come from the underlying primitives.
 */
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import * as React from "react";

/* -------------------------------------------------------------------------- */
/*  Panel / surface                                                           */
/* -------------------------------------------------------------------------- */

export type PanelTone = "card" | "raised" | "overlay";

const panelToneClass: Record<PanelTone, string> = {
  card: "bg-card text-card-foreground border-border",
  raised: "bg-raised text-raised-foreground border-border",
  overlay: "bg-overlay text-overlay-foreground border-border",
};

/**
 * A bordered IDE surface. Panels separate from each other by a small tonal step
 * and a 1px precision border, not by heavy shadows.
 */
export function Panel({
  tone = "card",
  className,
  ...props
}: React.ComponentProps<"div"> & { tone?: PanelTone }) {
  return (
    <div
      data-slot="altai-panel"
      data-tone={tone}
      className={cn(
        "flex flex-col overflow-hidden border",
        panelToneClass[tone],
        className,
      )}
      {...props}
    />
  );
}

/**
 * A compact panel header: a small-caps section label on the left and an
 * optional actions slot on the right (typically toolbar icon buttons).
 */
export function PanelHeader({
  label,
  meta,
  actions,
  className,
  children,
}: {
  label?: React.ReactNode;
  /** Mono technical metadata (count, branch, model id…). */
  meta?: React.ReactNode;
  actions?: React.ReactNode;
  className?: string;
  children?: React.ReactNode;
}) {
  return (
    <div
      className={cn(
        "flex h-9 shrink-0 items-center gap-2 border-b border-border-subtle px-2.5",
        className,
      )}
    >
      {label ? <SectionHeader className="flex-1">{label}</SectionHeader> : null}
      {children}
      {meta ? (
        <span className="altai-mono text-[10.5px] text-muted-foreground">
          {meta}
        </span>
      ) : null}
      {actions ? <div className="flex shrink-0 items-center gap-0.5">{actions}</div> : null}
    </div>
  );
}

/* -------------------------------------------------------------------------- */
/*  Toolbar icon button                                                       */
/* -------------------------------------------------------------------------- */

/**
 * Square icon-only toolbar button. Icons stay muted by default and become
 * prominent on hover / focus / active, matching the IDE density goal. `active`
 * lifts the surface (it does NOT turn lime — lime is reserved for primary
 * actions and selections).
 */
export const ToolbarIconButton = React.forwardRef<
  HTMLButtonElement,
  React.ComponentProps<typeof Button> & { active?: boolean }
>(function ToolbarIconButton({ active, className, size = "icon-sm", ...props }, ref) {
  return (
    <Button
      ref={ref}
      variant="ghost"
      size={size}
      data-active={active ? "" : undefined}
      aria-pressed={typeof active === "boolean" ? active : undefined}
      className={cn(
        // Default chrome density matches the macOS traffic-light row /
        // WindowControls (28×28). Callers can still override via `size` or
        // `className` (e.g. size-6 in denser panels).
        "size-7 shrink-0 rounded-md text-muted-foreground transition-colors hover:bg-accent hover:text-foreground focus-visible:text-foreground",
        active && "bg-accent text-foreground",
        className,
      )}
      {...props}
    />
  );
});

/* -------------------------------------------------------------------------- */
/*  Pill — nav / filter / status toggle                                       */
/* -------------------------------------------------------------------------- */

/**
 * Compact pill used for navigation, filters and status toggles. When `active`
 * it picks up the lime primary (the only place lime is used for selection
 * emphasis); otherwise it is muted.
 */
export function Pill({
  active = false,
  className,
  ...props
}: React.ComponentProps<"button"> & { active?: boolean }) {
  return (
    <button
      type="button"
      data-slot="altai-pill"
      aria-pressed={active}
      data-active={active ? "" : undefined}
      className={cn(
        "inline-flex h-6 shrink-0 cursor-pointer items-center gap-1 rounded-full border px-2.5 text-[11px] font-medium outline-none transition-colors",
        "focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/40",
        active
          ? "border-primary/40 bg-primary/12 text-primary dark:bg-primary/20"
          : "border-transparent text-muted-foreground hover:bg-accent hover:text-foreground",
        className,
      )}
      {...props}
    />
  );
}

/* -------------------------------------------------------------------------- */
/*  Section header                                                            */
/* -------------------------------------------------------------------------- */

/**
 * Small-caps section label. Pair with `PanelHeader`, or use standalone above a
 * grouped list of rows.
 */
export function SectionHeader({
  className,
  ...props
}: React.ComponentProps<"span">) {
  return (
    <span
      className={cn(
        "truncate text-[10.5px] font-semibold uppercase tracking-wide text-muted-foreground",
        className,
      )}
      {...props}
    />
  );
}

/* -------------------------------------------------------------------------- */
/*  Status dot / status badge                                                 */
/* -------------------------------------------------------------------------- */

export type StatusTone =
  | "success"
  | "warning"
  | "destructive"
  | "info"
  | "primary"
  | "muted";

const statusDotClass: Record<StatusTone, string> = {
  success: "bg-success",
  warning: "bg-warning",
  destructive: "bg-destructive",
  info: "bg-info",
  primary: "bg-primary",
  muted: "bg-muted-foreground/50",
};

/**
 * Tiny status indicator dot. Lime (`primary`) should be used sparingly — prefer
 * the matching semantic tone for status meaning.
 */
export function StatusDot({
  tone = "muted",
  pulse = false,
  className,
}: {
  tone?: StatusTone;
  pulse?: boolean;
  className?: string;
}) {
  return (
    <span
      aria-hidden
      className={cn(
        "inline-block size-1.5 shrink-0 rounded-full",
        statusDotClass[tone],
        pulse && "animate-pulse",
        className,
      )}
    />
  );
}

const statusSoftClass: Record<StatusTone, string> = {
  success:
    "bg-success/12 text-success ring-1 ring-inset ring-success/25 dark:bg-success/20",
  warning:
    "bg-warning/12 text-warning ring-1 ring-inset ring-warning/25 dark:bg-warning/20",
  destructive:
    "bg-destructive/12 text-destructive ring-1 ring-inset ring-destructive/25 dark:bg-destructive/20",
  info: "bg-info/12 text-info ring-1 ring-inset ring-info/25 dark:bg-info/20",
  primary:
    "bg-primary/12 text-primary ring-1 ring-inset ring-primary/25 dark:bg-primary/20",
  muted: "bg-muted text-muted-foreground ring-1 ring-inset ring-border",
};

/**
 * Compact status badge with a leading dot. Use for agent run states, todo
 * states, build/check results.
 */
export function StatusBadge({
  tone = "muted",
  pulse = false,
  className,
  children,
}: {
  tone?: StatusTone;
  pulse?: boolean;
  className?: string;
  children?: React.ReactNode;
}) {
  return (
    <span
      className={cn(
        "inline-flex h-5 items-center gap-1 rounded-full px-2 text-[10.5px] font-medium tabular-nums",
        statusSoftClass[tone],
        className,
      )}
    >
      <StatusDot tone={tone} pulse={pulse} />
      {children}
    </span>
  );
}

/* -------------------------------------------------------------------------- */
/*  Data / list row                                                           */
/* -------------------------------------------------------------------------- */

/**
 * A consistent list row. `selected` highlights with a raised surface (not lime
 * — lime is the indicator accent, applied by callers via a left rail or dot).
 */
export function DataRow({
  selected = false,
  className,
  ...props
}: React.ComponentProps<"div"> & { selected?: boolean }) {
  return (
    <div
      data-slot="altai-row"
      aria-selected={selected}
      className={cn(
        "flex min-w-0 items-center gap-2 px-2.5 py-1.5 text-[12px] transition-colors",
        selected ? "bg-accent text-accent-foreground" : "hover:bg-accent/60",
        className,
      )}
      {...props}
    />
  );
}

/* -------------------------------------------------------------------------- */
/*  Empty state                                                               */
/* -------------------------------------------------------------------------- */

/**
 * Centered empty / welcome / onboarding state. This is one of the few places
 * large display typography and the lime ambient glow are allowed.
 */
export function EmptyState({
  icon,
  title,
  description,
  action,
  glow = true,
  className,
}: {
  icon?: React.ReactNode;
  title: React.ReactNode;
  description?: React.ReactNode;
  action?: React.ReactNode;
  glow?: boolean;
  className?: string;
}) {
  return (
    <div
      className={cn(
        "relative flex flex-col items-center justify-center gap-3 px-8 py-10 text-center",
        className,
      )}
    >
      {glow ? <span className="altai-ambient-glow" aria-hidden /> : null}
      <div className="relative z-10 flex flex-col items-center gap-3">
        {icon ? (
          <div className="flex size-11 items-center justify-center rounded-xl border border-border bg-raised text-muted-foreground">
            {icon}
          </div>
        ) : null}
        <h2 className="text-lg font-semibold tracking-tight text-foreground">
          {title}
        </h2>
        {description ? (
          <p className="max-w-sm text-[12.5px] leading-relaxed text-muted-foreground">
            {description}
          </p>
        ) : null}
        {action ? <div className="mt-1">{action}</div> : null}
      </div>
    </div>
  );
}

/* -------------------------------------------------------------------------- */
/*  Plan step — agent plan / todo                                             */
/* -------------------------------------------------------------------------- */

export type PlanStepStatus = "pending" | "in_progress" | "completed" | "failed";

const planStatusTone: Record<PlanStepStatus, StatusTone> = {
  pending: "muted",
  in_progress: "info",
  completed: "success",
  failed: "destructive",
};

/**
 * A single agent plan / todo step with a status marker. In-progress steps pulse
 * so motion-capable users notice activity; this is reduced automatically under
 * prefers-reduced-motion (handled globally in globals.css).
 */
export function PlanStep({
  status,
  label,
  meta,
  className,
}: {
  status: PlanStepStatus;
  label: React.ReactNode;
  meta?: React.ReactNode;
  className?: string;
}) {
  const tone = planStatusTone[status];
  return (
    <div
      data-slot="altai-plan-step"
      data-status={status}
      className={cn(
        "flex items-start gap-2 py-1 text-[12px] leading-relaxed",
        status === "completed" ? "text-muted-foreground" : "text-foreground",
        className,
      )}
    >
      <StatusDot
        tone={tone}
        pulse={status === "in_progress"}
        className="mt-1.5"
      />
      <span className="min-w-0 flex-1 break-words">{label}</span>
      {meta ? (
        <span className="altai-mono shrink-0 text-[10.5px] text-muted-foreground">
          {meta}
        </span>
      ) : null}
    </div>
  );
}

/* -------------------------------------------------------------------------- */
/*  Status tone helpers (for ad-hoc soft fills)                               */
/* -------------------------------------------------------------------------- */

/** Soft background + text for a semantic tone, e.g. inline status chips. */
export function statusSoft(tone: StatusTone): string {
  return statusSoftClass[tone];
}

/** Pure semantic foreground class for a tone (text-only usage). */
export function statusText(tone: StatusTone): string {
  const map: Record<StatusTone, string> = {
    success: "text-success",
    warning: "text-warning",
    destructive: "text-destructive",
    info: "text-info",
    primary: "text-primary",
    muted: "text-muted-foreground",
  };
  return map[tone];
}
