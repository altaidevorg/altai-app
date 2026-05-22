import { useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

/**
 * Restore keyboard focus to the last-focused element when the window
 * regains foreground focus (Cmd+Tab back, Alt+Tab back, dock-icon click).
 *
 * Without this, the WebView dumps focus on `document.body` whenever the
 * window comes back to the front. The visible cursor goes blank, the
 * screen reader narrates the window title but no focused element, and
 * keyboard users have to Tab from scratch every time they switch apps.
 *
 * Pattern:
 *  - capture-phase `focusin` listener tracks the most-recent focused
 *    element while the window is active.
 *  - Tauri `onFocusChanged` fires on every foreground transition. On
 *    focus regain we wait one frame (WebView's own focus-handling races
 *    with ours otherwise) and re-focus the tracked element if it's
 *    still in the DOM and rendered.
 */
export function useRestoreFocusOnReturn(): void {
  useEffect(() => {
    let lastFocused: HTMLElement | null = null;

    const onFocusIn = (e: FocusEvent) => {
      const target = e.target;
      if (target instanceof HTMLElement && target !== document.body) {
        lastFocused = target;
      }
    };
    document.addEventListener("focusin", onFocusIn, true);

    const win = getCurrentWindow();
    const unlistenPromise = win.onFocusChanged((event) => {
      const focused = event.payload as boolean;
      if (!focused) return;
      requestAnimationFrame(() => {
        const el = lastFocused;
        if (!el || !document.contains(el)) return;
        // Skip if the element is hidden — focus() on a display:none node
        // is silently a no-op and leaves the SR pointing at nothing.
        if (el.offsetParent === null && el !== document.documentElement) {
          return;
        }
        try {
          el.focus({ preventScroll: false });
        } catch {
          // Element may have been re-rendered / replaced; harmless.
        }
      });
    });

    return () => {
      document.removeEventListener("focusin", onFocusIn, true);
      void unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);
}
