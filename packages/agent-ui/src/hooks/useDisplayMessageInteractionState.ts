/**
 * Headless interaction state for flat display-message lists (A6.55).
 * Hosts keep clipboard / ports handlers; package owns edit + busy ids.
 */

import { useCallback, useRef, useState } from "react";

export type DisplayMessageInteractionState = {
  editingId: string | null;
  draft: string;
  setDraft: (value: string) => void;
  beginEdit: (messageId: string, content: string) => void;
  cancelEdit: () => void;
  finishEdit: () => void;
  openingId: string | null;
  beginOpen: (messageId: string) => void;
  endOpen: () => void;
  copiedId: string | null;
  /** Mark copied with auto-clear after `clearMs` (default 1500). */
  markCopied: (messageId: string, clearMs?: number) => void;
};

export function useDisplayMessageInteractionState(): DisplayMessageInteractionState {
  const [editingId, setEditingId] = useState<string | null>(null);
  const [draft, setDraft] = useState("");
  const [openingId, setOpeningId] = useState<string | null>(null);
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const copyTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const beginEdit = useCallback((messageId: string, content: string) => {
    setEditingId(messageId);
    setDraft(content);
  }, []);

  const cancelEdit = useCallback(() => {
    setEditingId(null);
  }, []);

  const finishEdit = useCallback(() => {
    setEditingId(null);
  }, []);

  const beginOpen = useCallback((messageId: string) => {
    setOpeningId(messageId);
  }, []);

  const endOpen = useCallback(() => {
    setOpeningId(null);
  }, []);

  const markCopied = useCallback((messageId: string, clearMs = 1500) => {
    setCopiedId(messageId);
    if (copyTimer.current) {
      clearTimeout(copyTimer.current);
    }
    copyTimer.current = setTimeout(() => {
      setCopiedId((id) => (id === messageId ? null : id));
      copyTimer.current = null;
    }, clearMs);
  }, []);

  return {
    editingId,
    draft,
    setDraft,
    beginEdit,
    cancelEdit,
    finishEdit,
    openingId,
    beginOpen,
    endOpen,
    copiedId,
    markCopied,
  };
}
