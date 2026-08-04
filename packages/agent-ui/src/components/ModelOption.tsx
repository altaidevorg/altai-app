import { Tick01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon, type IconSvgElement } from "@hugeicons/react";
import { cn } from "../lib/cn.js";

export type ModelOptionProps = {
  modelLabel: string;
  label?: string;
  detail?: string;
  providerIcon?: IconSvgElement;
  selected: boolean;
  active: boolean;
  showProvider: boolean;
  pinned?: boolean;
  domId?: string;
  onClick: () => void;
  onTogglePin?: () => void;
};

/**
 * Model list option row for the model dropdown. Shows provider icon, label,
 * detail, selected checkmark, and an optional pin toggle. The host owns the
 * model data and pin state.
 */
export function ModelOption({
  modelLabel,
  label,
  detail,
  providerIcon,
  selected,
  active,
  showProvider,
  pinned = false,
  domId,
  onClick,
  onTogglePin,
}: ModelOptionProps) {
  return (
    <div className="group/model-option relative mx-1 my-0.5">
      <button
        type="button"
        id={label ? undefined : domId}
        role="option"
        aria-selected={selected}
        data-active={active || undefined}
        onClick={onClick}
        className={cn(
          "flex w-full items-center gap-2 rounded-md px-2 py-1.5 pr-8 text-left",
          selected
            ? "bg-foreground/[0.085] text-popover-foreground"
            : active
              ? "bg-foreground/[0.065] text-popover-foreground"
              : "text-popover-foreground hover:bg-foreground/[0.055]",
        )}
      >
        {showProvider && providerIcon ? (
          <HugeiconsIcon
            icon={providerIcon}
            size={13}
            strokeWidth={1.5}
            className="shrink-0 text-muted-foreground/70"
          />
        ) : null}
        <span className="min-w-0 flex-1">
          <span className="block truncate text-[12px] font-medium">
            {label ?? modelLabel}
          </span>
          {detail ? (
            <span className="block truncate text-[10px] text-muted-foreground">
              {detail}
            </span>
          ) : null}
        </span>
        {selected ? (
          <HugeiconsIcon icon={Tick01Icon} size={13} strokeWidth={2} className="shrink-0" />
        ) : null}
      </button>
      {onTogglePin ? (
        <button
          type="button"
          aria-label={`${pinned ? "Unpin" : "Pin"} ${modelLabel}`}
          title={pinned ? "Unpin model" : "Pin model"}
          onClick={(event) => {
            event.stopPropagation();
            onTogglePin();
          }}
          className={cn(
            "absolute top-1/2 right-1 -translate-y-1/2 rounded-md px-1.5 py-0.5 text-[10px] transition-colors",
            pinned
              ? "text-foreground"
              : "text-muted-foreground opacity-0 group-hover/model-option:opacity-100 hover:bg-foreground/[0.08] hover:text-foreground",
          )}
        >
          {pinned ? "Pinned" : "Pin"}
        </button>
      ) : null}
    </div>
  );
}
