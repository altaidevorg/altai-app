import {
  ArrowDown01Icon,
  CheckmarkCircle02Icon,
  Edit02Icon,
  Route01Icon,
  Settings01Icon,
  ShieldEnergyIcon,
  Tick02Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import {
  PERMISSION_MODES,
  type PermissionMode,
} from "@altai/host-contract";
import { useEffect, useId, useRef, useState } from "react";
import { cn } from "../lib/cn.js";

export const PERMISSION_MODE_LABELS: Record<PermissionMode, string> = {
  ask: "Ask before edit",
  "auto-edit": "Edit automatically",
  plan: "Plan mode",
  bypass: "Bypass permissions",
};

export const PERMISSION_MODE_DESCRIPTIONS: Record<PermissionMode, string> = {
  ask: "Approve every file edit, write, and shell command before it runs.",
  "auto-edit":
    "Auto-approve file edits and writes. Shell commands still require approval.",
  plan: "Read-only: the agent can explore, search, and plan, but cannot edit files. Shell commands still require approval.",
  bypass:
    "Auto-approve everything, including shell commands. Use only in sandboxed environments.",
};

/**
 * Safety invariant: stale `"bypass"` falls back to `"ask"` when bypass is not
 * unlocked. Hosts should reuse this for send-flow guards too.
 */
export function effectivePermissionMode(
  mode: PermissionMode,
  bypassEnabled: boolean,
): PermissionMode {
  return mode === "bypass" && !bypassEnabled ? "ask" : mode;
}

/** Modes shown in the switcher menu (capability-gated bypass). */
export function visiblePermissionModes(
  showBypass = true,
): readonly PermissionMode[] {
  return showBypass
    ? PERMISSION_MODES
    : PERMISSION_MODES.filter((m) => m !== "bypass");
}

const ICONS: Record<PermissionMode, typeof CheckmarkCircle02Icon> = {
  ask: CheckmarkCircle02Icon,
  "auto-edit": Edit02Icon,
  plan: Route01Icon,
  bypass: ShieldEnergyIcon,
};

const MODE_COLORS: Record<
  PermissionMode,
  { trigger: string; icon: string; label: string }
> = {
  ask: {
    trigger: "text-success hover:text-success",
    icon: "text-success",
    label: "text-success",
  },
  "auto-edit": {
    trigger: "text-info hover:text-info",
    icon: "text-info",
    label: "text-info",
  },
  plan: {
    trigger: "text-warning hover:text-warning",
    icon: "text-warning",
    label: "text-warning",
  },
  bypass: {
    trigger: "text-destructive hover:text-destructive",
    icon: "text-destructive",
    label: "text-destructive",
  },
};

export type PermissionModeSwitcherProps = {
  mode: PermissionMode;
  bypassEnabled: boolean;
  onSelectMode: (mode: PermissionMode) => void;
  /** Opens host settings / permission management. */
  onManagePermissions?: () => void;
  variant?: "toolbar" | "toolbar-icon";
  /** When false, omit the bypass option (capability-gated hosts). */
  showBypass?: boolean;
};

/**
 * Composer permission-mode control. Hosts supply current mode + callbacks;
 * no settings store or window APIs.
 */
export function PermissionModeSwitcher({
  mode,
  bypassEnabled,
  onSelectMode,
  onManagePermissions,
  variant = "toolbar",
  showBypass = true,
}: PermissionModeSwitcherProps) {
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const menuId = useId();
  const effectiveMode = effectivePermissionMode(mode, bypassEnabled);
  const ActiveIcon = ICONS[effectiveMode];
  const activeColors = MODE_COLORS[effectiveMode];
  const isIconOnly = variant === "toolbar-icon";
  const visibleModes = visiblePermissionModes(showBypass);

  useEffect(() => {
    if (!open) return;
    const onPointerDown = (event: PointerEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) {
        setOpen(false);
      }
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpen(false);
    };
    document.addEventListener("pointerdown", onPointerDown);
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown);
      document.removeEventListener("keydown", onKeyDown);
    };
  }, [open]);

  return (
    <div ref={rootRef} className="relative inline-flex min-w-0">
      <button
        type="button"
        aria-haspopup="menu"
        aria-expanded={open}
        aria-controls={menuId}
        aria-label={`Permission mode: ${PERMISSION_MODE_LABELS[effectiveMode]}`}
        title={`Permission mode: ${PERMISSION_MODE_LABELS[effectiveMode]}`}
        onClick={() => setOpen((value) => !value)}
        className={cn(
          "group inline-flex h-7 min-w-0 max-w-[10rem] items-center gap-1.5 rounded-md border border-transparent px-2 text-[11.5px] transition-colors outline-none select-none",
          "hover:bg-foreground/[0.055] focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/30",
          activeColors.trigger,
        )}
      >
        <HugeiconsIcon
          icon={ActiveIcon}
          size={13}
          strokeWidth={1.75}
          className="shrink-0"
          aria-hidden="true"
        />
        {!isIconOnly ? (
          <>
            <span className="truncate font-medium">
              {PERMISSION_MODE_LABELS[effectiveMode]}
            </span>
            <HugeiconsIcon
              icon={ArrowDown01Icon}
              size={11}
              strokeWidth={2}
              className="shrink-0 opacity-60 transition-opacity group-hover:opacity-90"
            />
          </>
        ) : null}
      </button>

      {open ? (
        <div
          id={menuId}
          role="menu"
          aria-label="Permissions"
          className="absolute bottom-full left-0 z-50 mb-1.5 w-[min(22rem,calc(100vw-1rem))] overflow-hidden rounded-md border border-border bg-popover text-popover-foreground shadow-md"
        >
          <div className="px-2 pt-1.5 pb-1 text-[10px] font-medium tracking-wide text-muted-foreground uppercase">
            Permissions
          </div>
          {visibleModes.map((m) => {
            const Icon = ICONS[m];
            const isActive = m === effectiveMode;
            const danger = m === "bypass";
            const colors = MODE_COLORS[m];
            return (
              <button
                key={m}
                type="button"
                role="menuitemradio"
                aria-checked={isActive}
                onClick={() => {
                  onSelectMode(m);
                  setOpen(false);
                }}
                className={cn(
                  "flex w-full items-start gap-2 px-2 py-1.5 pr-2 text-left text-[12px] outline-none",
                  "hover:bg-foreground/[0.06] focus-visible:bg-foreground/[0.06]",
                  isActive && "bg-foreground/[0.085]",
                  danger && "hover:bg-destructive/10 focus-visible:bg-destructive/10",
                )}
              >
                <HugeiconsIcon
                  icon={Icon}
                  size={13}
                  strokeWidth={1.75}
                  className={cn("mt-0.5 shrink-0", colors.icon)}
                />
                <span className="flex min-w-0 flex-1 flex-col">
                  <span className={colors.label}>
                    {PERMISSION_MODE_LABELS[m]}
                  </span>
                  <span className="line-clamp-2 text-[10.5px] text-muted-foreground">
                    {PERMISSION_MODE_DESCRIPTIONS[m]}
                  </span>
                </span>
                {isActive ? (
                  <HugeiconsIcon
                    icon={Tick02Icon}
                    size={12}
                    strokeWidth={2}
                    className={cn("mt-0.5 shrink-0", colors.icon)}
                  />
                ) : null}
              </button>
            );
          })}
          {onManagePermissions ? (
            <>
              <div className="my-1 h-px bg-border" role="separator" />
              <button
                type="button"
                role="menuitem"
                onClick={() => {
                  onManagePermissions();
                  setOpen(false);
                }}
                className="flex w-full items-center gap-2 px-2 py-1.5 text-left text-[12px] text-muted-foreground outline-none hover:bg-foreground/[0.06] focus-visible:bg-foreground/[0.06]"
              >
                <HugeiconsIcon
                  icon={Settings01Icon}
                  size={12}
                  strokeWidth={1.75}
                />
                Manage permissions…
              </button>
            </>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}
