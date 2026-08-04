import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { cn } from "@/lib/utils";
import { SessionRow } from "@altai/agent-ui";
import {
  Clock01Icon,
  Search01Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import type { UIMessage } from "ai";
import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { loadMessages, type SessionMeta } from "../lib/sessions";
import { useChatStore } from "../store/chatStore";

type DateGroup = {
  label: string;
  items: SessionMeta[];
};

const DAY_MS = 24 * 60 * 60 * 1000;

function startOfDay(ts: number): number {
  const d = new Date(ts);
  d.setHours(0, 0, 0, 0);
  return d.getTime();
}

function bucketFor(ts: number, nowDay: number): string {
  const day = startOfDay(ts);
  if (day === nowDay) return "Today";
  if (day === nowDay - DAY_MS) return "Yesterday";
  if (day > nowDay - 7 * DAY_MS) return "Previous 7 days";
  if (day > nowDay - 30 * DAY_MS) return "Previous 30 days";
  return "Older";
}

const GROUP_ORDER = [
  "Today",
  "Yesterday",
  "Previous 7 days",
  "Previous 30 days",
  "Older",
];

function extractSnippet(messages: UIMessage[]): string {
  for (let i = messages.length - 1; i >= 0; i--) {
    const m = messages[i];
    for (const p of m.parts) {
      if (p.type !== "text") continue;
      const raw = (p as { text?: string }).text ?? "";
      const cleaned = raw
        .replace(/<terminal-context[\s\S]*?<\/terminal-context>\s*/g, "")
        .replace(/<git-diff[\s\S]*?<\/git-diff>\s*/g, "")
        .replace(/<folder[\s\S]*?<\/folder>\s*/g, "")
        .replace(/<selection[\s\S]*?<\/selection>\s*/g, "")
        .replace(/<file[\s\S]*?<\/file>\s*/g, "")
        .replace(/\s+/g, " ")
        .trim();
      if (cleaned) {
        return cleaned.length > 90 ? `${cleaned.slice(0, 90)}…` : cleaned;
      }
    }
  }
  return "";
}

export function ChatHistory() {
  const [open, setOpen] = useState(false);
  const sessions = useChatStore((s) => s.sessions);
  const activeId = useChatStore((s) => s.activeSessionId);
  const switchSession = useChatStore((s) => s.switchSession);
  const deleteSession = useChatStore((s) => s.deleteSession);
  const renameSession = useChatStore((s) => s.renameSession);

  const [search, setSearch] = useState("");
  const [snippets, setSnippets] = useState<Record<string, string>>({});
  const loadedRef = useRef<Set<string>>(new Set());
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState("");
  const renameInputRef = useRef<HTMLInputElement>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);

  // Lazy load snippets when popover opens; cache so re-opens are instant.
  useEffect(() => {
    if (!open) return;
    let cancelled = false;
    const load = async () => {
      for (const s of sessions) {
        if (loadedRef.current.has(s.id)) continue;
        loadedRef.current.add(s.id);
        const msgs = await loadMessages(s.id);
        if (cancelled) return;
        const snippet = extractSnippet(msgs ?? []);
        setSnippets((prev) =>
          prev[s.id] === snippet ? prev : { ...prev, [s.id]: snippet },
        );
      }
    };
    void load();
    return () => {
      cancelled = true;
    };
  }, [open, sessions]);

  // Reset search and rename state when popover closes.
  useEffect(() => {
    if (!open) {
      setSearch("");
      setRenamingId(null);
      setRenameValue("");
    } else {
      requestAnimationFrame(() => searchInputRef.current?.focus());
    }
  }, [open]);

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

  const filtered = useMemo(() => {
    const q = search.trim().toLowerCase();
    if (!q) return sessions;
    return sessions.filter((s) => {
      const title = (s.title || "New chat").toLowerCase();
      if (title.includes(q)) return true;
      const snippet = snippets[s.id] ?? "";
      return snippet.toLowerCase().includes(q);
    });
  }, [sessions, search, snippets]);

  const groups = useMemo<DateGroup[]>(() => {
    const nowDay = startOfDay(Date.now());
    const map = new Map<string, SessionMeta[]>();
    for (const s of filtered) {
      const label = bucketFor(s.updatedAt, nowDay);
      const arr = map.get(label) ?? [];
      arr.push(s);
      map.set(label, arr);
    }
    for (const arr of map.values()) {
      arr.sort((a, b) => b.updatedAt - a.updatedAt);
    }
    return GROUP_ORDER.filter((label) => map.has(label)).map((label) => ({
      label,
      items: map.get(label)!,
    }));
  }, [filtered]);

  const handlePick = useCallback(
    (id: string) => {
      switchSession(id);
      setOpen(false);
    },
    [switchSession],
  );

  const handleStartRename = useCallback((session: SessionMeta) => {
    setRenamingId(session.id);
    setRenameValue(session.title || "New chat");
  }, []);

  const handleCommitRename = useCallback(() => {
    if (!renamingId) return;
    const trimmed = renameValue.trim();
    if (trimmed) renameSession(renamingId, trimmed);
    setRenamingId(null);
    setRenameValue("");
  }, [renamingId, renameValue, renameSession]);

  const handleCancelRename = useCallback(() => {
    setRenamingId(null);
    setRenameValue("");
  }, []);

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <button
          type="button"
          title="Chat history"
          aria-label="Chat history"
          className={cn(
            "inline-flex size-6 shrink-0 items-center justify-center rounded-md",
            "text-muted-foreground transition-colors hover:bg-accent hover:text-foreground",
            open && "bg-accent text-foreground",
          )}
        >
          <HugeiconsIcon icon={Clock01Icon} size={13} strokeWidth={1.75} />
        </button>
      </PopoverTrigger>
      <PopoverContent
        side="bottom"
        align="end"
        sideOffset={6}
        collisionPadding={8}
        className="flex w-[min(22rem,calc(100vw-1rem))] max-h-[28rem] flex-col gap-0 overflow-hidden rounded-xl p-0"
      >
        <div className="flex shrink-0 items-center gap-2 border-b border-border/60 px-3 py-2">
          <HugeiconsIcon
            icon={Search01Icon}
            size={13}
            strokeWidth={1.75}
            className="shrink-0 text-muted-foreground/80"
          />
          <input
            ref={searchInputRef}
            aria-label="Search chat history"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Escape") {
                if (search) {
                  e.stopPropagation();
                  setSearch("");
                }
              }
            }}
            placeholder="Search chat history…"
            className="w-full bg-transparent text-[12px] outline-none placeholder:text-muted-foreground/60"
          />
        </div>

        <div className="min-h-0 flex-1 overflow-y-auto py-1">
          {groups.length === 0 ? (
            <div className="px-3 py-8 text-center text-[11px] text-muted-foreground/70">
              {search ? "No chats match." : "No chats yet."}
            </div>
          ) : (
            groups.map((group) => (
              <div key={group.label} className="px-1">
                <div className="px-2 pt-2 pb-1 text-[10px] font-medium uppercase tracking-wide text-muted-foreground/80">
                  {group.label}
                </div>
                {group.items.map((session) => (
                  <SessionRow
                    key={session.id}
                    title={session.title}
                    snippet={snippets[session.id]}
                    active={session.id === activeId}
                    renaming={renamingId === session.id}
                    renameValue={renameValue}
                    onPick={() => handlePick(session.id)}
                    onStartRename={() => handleStartRename(session)}
                    onCommitRename={handleCommitRename}
                    onCancelRename={handleCancelRename}
                    onRenameValueChange={setRenameValue}
                    onDelete={() => deleteSession(session.id)}
                    renameInputRef={renameInputRef}
                  />
                ))}
              </div>
            ))
          )}
        </div>
      </PopoverContent>
    </Popover>
  );
}



