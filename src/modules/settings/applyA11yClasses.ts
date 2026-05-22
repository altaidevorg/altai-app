import { useEffect } from "react";
import { usePreferencesStore } from "./preferences";

/**
 * Mirror accessibility preferences onto the document root as data attributes
 * and class names. CSS in `globals.css` reads these to apply reduce-motion,
 * high-contrast, larger-text, stronger-focus, and link-underline overrides.
 *
 * Idempotent — call once from the app shell (`App.tsx`). Hot updates work
 * because each preference is subscribed individually via `usePreferencesStore`.
 */
export function useApplyA11yClasses(): void {
  const reduceMotion = usePreferencesStore((s) => s.reduceMotion);
  const highContrast = usePreferencesStore((s) => s.highContrast);
  const largerText = usePreferencesStore((s) => s.largerText);
  const underlineLinks = usePreferencesStore((s) => s.underlineLinks);
  const focusRing = usePreferencesStore((s) => s.focusRing);
  const showSkipLinks = usePreferencesStore((s) => s.showSkipLinks);

  useEffect(() => {
    const root = document.documentElement;

    // `data-a11y-motion` short-circuits the @media query in either direction.
    // "system" leaves it alone; "always"/"never" force a value.
    if (reduceMotion === "always") root.setAttribute("data-a11y-motion", "reduce");
    else if (reduceMotion === "never") root.setAttribute("data-a11y-motion", "no-preference");
    else root.removeAttribute("data-a11y-motion");

    root.classList.toggle("a11y-high-contrast", highContrast);
    root.classList.toggle("a11y-larger-text", largerText);
    root.classList.toggle("a11y-underline-links", underlineLinks);
    root.classList.toggle("a11y-strong-focus", focusRing === "strong");
    root.classList.toggle("a11y-show-skip-links", showSkipLinks);
  }, [reduceMotion, highContrast, largerText, underlineLinks, focusRing, showSkipLinks]);
}
