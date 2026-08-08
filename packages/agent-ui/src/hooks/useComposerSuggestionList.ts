/**
 * Headless composer suggestion list state for slash/snippet popovers (A6.66).
 * Hosts supply match catalogs via getMatches; package owns open + keyboard state.
 */

import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { detectSlashOrSnippetTrigger } from "../lib/composerTriggers.js";
import {
  resolveComposerSuggestionKeyAction,
  resolveComposerSuggestionOpen,
} from "../lib/composerSuggestionKeyboard.js";

export type UseComposerSuggestionListOptions<T> = {
  prompt: string;
  cursor: number;
  prefix: "/" | "#";
  disabled?: boolean;
  /** Resolve suggestions for the current trigger query (capped by host). */
  getMatches: (query: string) => readonly T[];
  onPick: (item: T) => void;
};

export type ComposerSuggestionListController<T> = {
  open: boolean;
  query: string;
  activeIndex: number;
  setActiveIndex: (index: number) => void;
  forceClose: () => void;
  isOpen: () => boolean;
  handleKeyDown: (key: string) => boolean;
  matches: readonly T[];
};

export function useComposerSuggestionList<T>({
  prompt,
  cursor,
  prefix,
  disabled = false,
  getMatches,
  onPick,
}: UseComposerSuggestionListOptions<T>): ComposerSuggestionListController<T> {
  const [forceClosed, setForceClosed] = useState(false);
  const [activeIndex, setActiveIndex] = useState(0);

  const trigger = useMemo(() => {
    if (disabled) return null;
    return detectSlashOrSnippetTrigger(prompt, cursor);
  }, [prompt, cursor, disabled]);

  const { open, query } = resolveComposerSuggestionOpen({
    trigger,
    forceClosed,
    prefix,
  });

  const matches = useMemo(
    () => (open ? getMatches(query) : []),
    [open, query, getMatches],
  );

  useEffect(() => {
    if (!open) {
      setForceClosed(false);
      setActiveIndex(0);
    }
  }, [open]);

  useEffect(() => {
    setActiveIndex(0);
  }, [query]);

  const stateRef = useRef({ matches, activeIndex, onPick });
  stateRef.current = { matches, activeIndex, onPick };

  const forceClose = useCallback(() => setForceClosed(true), []);

  const isOpen = useCallback(() => open && Boolean(trigger), [open, trigger]);

  const handleKeyDown = useCallback(
    (key: string) => {
      if (!open) return false;
      const snap = stateRef.current;
      const action = resolveComposerSuggestionKeyAction(key, {
        matchCount: snap.matches.length,
        activeIndex: snap.activeIndex,
      });
      if (action.type === "close") {
        setForceClosed(true);
        return true;
      }
      if (action.type === "ignore") return false;
      if (action.type === "move") {
        setActiveIndex(action.index);
        return true;
      }
      const item = snap.matches[action.index];
      if (item !== undefined) {
        snap.onPick(item);
        setForceClosed(true);
        return true;
      }
      return false;
    },
    [open],
  );

  return {
    open,
    query,
    activeIndex,
    setActiveIndex,
    forceClose,
    isOpen,
    handleKeyDown,
    matches,
  };
}
