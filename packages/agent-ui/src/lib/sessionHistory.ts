export type SessionHistoryItem = {
  id: string;
  title: string;
  updatedAt: number;
  snippet?: string;
};

export type SessionHistoryGroup = {
  label: string;
  items: SessionHistoryItem[];
};

const DAY_MS = 24 * 60 * 60 * 1000;

export const SESSION_HISTORY_GROUP_ORDER = [
  "Today",
  "Yesterday",
  "Previous 7 days",
  "Previous 30 days",
  "Older",
] as const;

export function startOfDay(ts: number): number {
  const d = new Date(ts);
  d.setHours(0, 0, 0, 0);
  return d.getTime();
}

export function sessionHistoryBucket(
  updatedAt: number,
  nowDay: number,
): (typeof SESSION_HISTORY_GROUP_ORDER)[number] {
  const day = startOfDay(updatedAt);
  if (day === nowDay) return "Today";
  if (day === nowDay - DAY_MS) return "Yesterday";
  if (day > nowDay - 7 * DAY_MS) return "Previous 7 days";
  if (day > nowDay - 30 * DAY_MS) return "Previous 30 days";
  return "Older";
}

/**
 * Group sessions into recency buckets (Today → Older). Newest first within
 * each bucket. Host filters drafts/search before calling this.
 */
export function groupSessionsByRecency(
  sessions: SessionHistoryItem[],
  nowMs: number = Date.now(),
): SessionHistoryGroup[] {
  const nowDay = startOfDay(nowMs);
  const map = new Map<string, SessionHistoryItem[]>();
  for (const session of sessions) {
    const label = sessionHistoryBucket(session.updatedAt, nowDay);
    const arr = map.get(label) ?? [];
    arr.push(session);
    map.set(label, arr);
  }
  for (const arr of map.values()) {
    arr.sort((a, b) => b.updatedAt - a.updatedAt);
  }
  return SESSION_HISTORY_GROUP_ORDER.filter((label) => map.has(label)).map(
    (label) => ({
      label,
      items: map.get(label)!,
    }),
  );
}

/**
 * Map host session rows + snippets into history list items.
 * Empty/null titles fall back to `defaultTitle`.
 */
export function sessionHistoryItemsFromSessions(
  sessions: readonly {
    id: string;
    title?: string | null;
    updatedAt: number;
  }[],
  snippets: Readonly<Record<string, string | undefined>>,
  defaultTitle = "New chat",
): SessionHistoryItem[] {
  return sessions.map((session) => ({
    id: session.id,
    title: session.title || defaultTitle,
    updatedAt: session.updatedAt,
    snippet: snippets[session.id],
  }));
}

/**
 * Commit-ready rename title, or null when blank after trim.
 */
export function trimmedSessionRenameTitle(value: string): string | null {
  const trimmed = value.trim();
  return trimmed ? trimmed : null;
}
