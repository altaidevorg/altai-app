import {
  ChatHistoryPanel as SharedChatHistoryPanel,
  extractSessionSnippet,
  filterSessionsForHistorySearch,
  groupSessionsByRecency,
  hasConversationContent,
} from "@altai/agent-ui";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { loadMessages } from "../lib/sessions";
import { useChatStore } from "../store/chatStore";

/**
 * Desktop bridge for the shared chat history panel. Owns store mutations,
 * transcript snippet loading, and draft filtering; UI comes from agent-ui.
 */
export function ChatHistoryPanel({
  onClose,
  autoFocusSearch = false,
}: {
  onClose: () => void;
  autoFocusSearch?: boolean;
}) {
  const sessions = useChatStore((s) => s.sessions);
  const activeId = useChatStore((s) => s.activeSessionId);
  const switchSession = useChatStore((s) => s.switchSession);
  const newSession = useChatStore((s) => s.newSession);
  const deleteSession = useChatStore((s) => s.deleteSession);
  const renameSession = useChatStore((s) => s.renameSession);

  const [search, setSearch] = useState("");
  const [snippets, setSnippets] = useState<Record<string, string>>({});
  const [hasContent, setHasContent] = useState<Record<string, boolean>>({});
  const loadedRef = useRef<Map<string, number>>(new Map());
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState("");
  const renameInputRef = useRef<HTMLInputElement | null>(null);
  const searchInputRef = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    if (!autoFocusSearch) return;
    const frame = requestAnimationFrame(() => searchInputRef.current?.focus());
    return () => cancelAnimationFrame(frame);
  }, [autoFocusSearch]);

  // Lazy load snippets so each row can show a preview of the conversation.
  useEffect(() => {
    let cancelled = false;
    const load = async () => {
      for (const s of sessions) {
        if (loadedRef.current.get(s.id) === s.updatedAt) continue;
        loadedRef.current.set(s.id, s.updatedAt);
        const msgs = await loadMessages(s.id);
        if (cancelled) return;
        const snippet = extractSessionSnippet(msgs ?? []);
        const containsContent = hasConversationContent(msgs ?? []);
        setSnippets((prev) =>
          prev[s.id] === snippet ? prev : { ...prev, [s.id]: snippet },
        );
        setHasContent((prev) =>
          prev[s.id] === containsContent
            ? prev
            : { ...prev, [s.id]: containsContent },
        );
      }
    };
    void load();
    return () => {
      cancelled = true;
    };
  }, [sessions]);

  useEffect(() => {
    if (renamingId) {
      requestAnimationFrame(() => {
        const el = renameInputRef.current;
        if (el) {
          el.focus();
          el.select();
        }
      });
    }
  }, [renamingId]);

  const filtered = useMemo(
    () =>
      filterSessionsForHistorySearch(sessions, {
        query: search,
        hasContent,
        snippets,
      }),
    [hasContent, sessions, search, snippets],
  );

  const groups = useMemo(
    () =>
      groupSessionsByRecency(
        filtered.map((s) => ({
          id: s.id,
          title: s.title || "New chat",
          updatedAt: s.updatedAt,
          snippet: snippets[s.id],
        })),
      ),
    [filtered, snippets],
  );

  const handlePick = useCallback(
    (id: string) => {
      switchSession(id);
      onClose();
    },
    [switchSession, onClose],
  );

  const handleNew = useCallback(() => {
    newSession();
    onClose();
  }, [newSession, onClose]);

  const commitRename = useCallback(() => {
    if (!renamingId) return;
    const trimmed = renameValue.trim();
    if (trimmed) renameSession(renamingId, trimmed);
    setRenamingId(null);
    setRenameValue("");
  }, [renamingId, renameValue, renameSession]);

  return (
    <SharedChatHistoryPanel
      groups={groups}
      activeId={activeId}
      search={search}
      onSearchChange={setSearch}
      onNewChat={handleNew}
      onPick={handlePick}
      onDelete={(id) => deleteSession(id)}
      renamingId={renamingId}
      renameValue={renameValue}
      onStartRename={(id, title) => {
        setRenamingId(id);
        setRenameValue(title);
      }}
      onCommitRename={commitRename}
      onCancelRename={() => {
        setRenamingId(null);
        setRenameValue("");
      }}
      onRenameValueChange={setRenameValue}
      renameInputRef={renameInputRef}
      searchInputRef={searchInputRef}
    />
  );
}
