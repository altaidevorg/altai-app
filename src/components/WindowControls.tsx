import { USE_CUSTOM_WINDOW_CONTROLS } from "@/lib/platform";
import { hasTauriWindowMetadata } from "@/lib/tauriWindow";
import { cn } from "@/lib/utils";
import {
  Cancel01Icon,
  Copy01Icon,
  MinusSignIcon,
  SquareIcon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useState } from "react";

type Props = {
  /** Render only the close button (used by the settings window). */
  closeOnly?: boolean;
};

export function WindowControls({ closeOnly = false }: Props) {
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    if (
      !USE_CUSTOM_WINDOW_CONTROLS ||
      closeOnly ||
      !hasTauriWindowMetadata()
    )
      return;

    let unlisten: (() => void) | undefined;
    try {
      const w = getCurrentWindow();
      void w.isMaximized().then(setMaximized).catch(() => undefined);
      void w
        .onResized(() => {
          void w.isMaximized().then(setMaximized).catch(() => undefined);
        })
        .then((un) => {
          unlisten = un;
        })
        .catch(() => undefined);
    } catch {
      return;
    }
    return () => unlisten?.();
  }, [closeOnly]);

  if (!USE_CUSTOM_WINDOW_CONTROLS && !closeOnly) return null;

  const runWindowAction = (
    action: (window: ReturnType<typeof getCurrentWindow>) => Promise<unknown>,
  ) => {
    if (!hasTauriWindowMetadata()) return;
    try {
      void action(getCurrentWindow()).catch(() => undefined);
    } catch {
      // The native window may disappear between render and click.
    }
  };

  return (
    <div className="flex h-full shrink-0 items-center gap-0.5 pr-1">
      {!closeOnly && (
        <>
          <CtlButton
            ariaLabel="Minimize"
            onClick={() => runWindowAction((w) => w.minimize())}
          >
            <HugeiconsIcon icon={MinusSignIcon} size={12} strokeWidth={2} />
          </CtlButton>
          <CtlButton
            ariaLabel={maximized ? "Restore" : "Maximize"}
            onClick={() => runWindowAction((w) => w.toggleMaximize())}
          >
            <HugeiconsIcon
              icon={maximized ? Copy01Icon : SquareIcon}
              size={12}
              strokeWidth={2}
            />
          </CtlButton>
        </>
      )}
      <CtlButton
        ariaLabel="Close"
        onClick={() => runWindowAction((w) => w.close())}
        danger
      >
        <HugeiconsIcon icon={Cancel01Icon} size={14} strokeWidth={2} />
      </CtlButton>
    </div>
  );
}

function CtlButton({
  ariaLabel,
  onClick,
  children,
  danger,
}: {
  ariaLabel: string;
  onClick: () => void;
  children: React.ReactNode;
  danger?: boolean;
}) {
  return (
    <button
      type="button"
      aria-label={ariaLabel}
      title={ariaLabel}
      onClick={onClick}
      className={cn(
        "grid size-6 place-items-center rounded-md text-muted-foreground transition-colors",
        danger
          ? "hover:bg-destructive/15 hover:text-destructive"
          : "hover:bg-accent hover:text-foreground",
      )}
    >
      {children}
    </button>
  );
}
