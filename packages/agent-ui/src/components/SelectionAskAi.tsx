import { useEffect, useState } from "react";
import { cn } from "../lib/cn.js";

export type SelectionAskAiProps = {
  x: number;
  y: number;
  onAsk: () => void;
  onDismiss: () => void;
  /** Host-formatted shortcut hint (e.g. "⌘L" / "Ctrl+L"). */
  shortcutLabel: string;
  /** Defaults to `window.innerWidth` when available. */
  viewportWidth?: number;
};

const W = 110;
const OFFSET = 32;

/**
 * Floating "Ask ALTAI" control shown near a text selection.
 * Hosts supply shortcut labeling and dismiss/ask wiring.
 */
export function SelectionAskAi({
  x,
  y,
  onAsk,
  onDismiss,
  shortcutLabel,
  viewportWidth,
}: SelectionAskAiProps) {
  const [entered, setEntered] = useState(false);

  useEffect(() => {
    const id = requestAnimationFrame(() => setEntered(true));
    return () => cancelAnimationFrame(id);
  }, []);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onDismiss();
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [onDismiss]);

  const width =
    viewportWidth ??
    (typeof window !== "undefined" ? window.innerWidth : W + 16);
  const top = Math.max(8, y - OFFSET);
  const left = Math.max(8, Math.min(x - W / 2, width - W - 8));

  return (
    <div
      data-selection-ask-ai
      style={{ top, left, width: W }}
      className={cn(
        "fixed z-50 origin-bottom transition duration-150 ease-out",
        entered ? "scale-100 opacity-100" : "scale-95 opacity-0",
      )}
    >
      <button
        type="button"
        onClick={(e) => {
          e.stopPropagation();
          onAsk();
        }}
        className="flex h-7 w-full items-center justify-between gap-1.5 rounded-md border border-border/60 bg-card/95 px-2 text-xs shadow-lg backdrop-blur-md hover:border-border hover:bg-accent"
      >
        <span>Ask ALTAI</span>
        <kbd className="inline-flex h-4 min-w-4 items-center justify-center rounded border border-border/70 bg-muted/60 px-1 font-sans text-[10px] text-muted-foreground">
          {shortcutLabel}
        </kbd>
      </button>
    </div>
  );
}
