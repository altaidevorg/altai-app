import {
  deriveChatTitleFromMessages,
  mapBackendMessageToTranscript,
  mergeRecoveredSessions,
  newSessionId as newSessionIdShared,
  sessionWorkspacePathForId,
} from "@altai/agent-ui";
import type { UIMessage } from "ai";
import { createAppStore } from "@/lib/appStore";

export type SessionMeta = {
  id: string;
  title: string;
  createdAt: number;
  updatedAt: number;
  /** Optional execution target. Missing/null means a project-free chat. */
  workspacePath?: string | null;
  workspaceKind?: "local" | "github" | null;
  repositoryUrl?: string | null;
};

/**
 * Desktop agent window vs IDE workspace window. Agent history is global to the
 * Desktop app; studio history is partitioned per open folder.
 */
export type ChatHistorySurface = "agent" | "studio";

const STORE_PATH = "altai-ai-sessions.json";
const LEGACY_KEY_SESSIONS = "sessions";
const LEGACY_KEY_ACTIVE = "activeId";
const LEGACY_KEY_DELETED = "deletedSessionIds";
const FLAT_STUDIO_SESSIONS = "sessions:studio";
const FLAT_STUDIO_ACTIVE = "activeId:studio";
const FLAT_STUDIO_DELETED = "deletedSessionIds:studio";
/** Studio open with no folder yet. */
export const STUDIO_SCOPE_NONE = "__none__";

/** Normalize folder paths used as studio history partition keys. */
export function canonicalizeHistoryWorkspacePath(path: string): string {
  return path.replace(/\\/g, "/").replace(/\/+$/, "");
}

export function studioScopeToken(scope: string | null): string {
  return scope ? canonicalizeHistoryWorkspacePath(scope) : STUDIO_SCOPE_NONE;
}

export function sessionsListKey(
  surface: ChatHistorySurface,
  scope: string | null = chatHistoryWorkspaceScope,
): string {
  if (surface === "agent") return "sessions:agent";
  return `sessions:studio:${studioScopeToken(scope)}`;
}

export function activeSessionKey(
  surface: ChatHistorySurface,
  scope: string | null = chatHistoryWorkspaceScope,
): string {
  if (surface === "agent") return "activeId:agent";
  return `activeId:studio:${studioScopeToken(scope)}`;
}

export function deletedSessionsKey(
  surface: ChatHistorySurface,
  scope: string | null = chatHistoryWorkspaceScope,
): string {
  if (surface === "agent") return "deletedSessionIds:agent";
  return `deletedSessionIds:studio:${studioScopeToken(scope)}`;
}

const messagesKey = (id: string) => `messages:${id}`;

const store = createAppStore(STORE_PATH, { defaults: {}, autoSave: 200 });

let chatHistorySurface: ChatHistorySurface = "agent";
/** Active Desktop IDE folder; ignored for the agent surface. */
let chatHistoryWorkspaceScope: string | null = null;

/** Bind subsequent load/save calls to Desktop (`agent`) or IDE (`studio`). */
export function setChatHistorySurface(surface: ChatHistorySurface): void {
  chatHistorySurface = surface;
}

export function getChatHistorySurface(): ChatHistorySurface {
  return chatHistorySurface;
}

/** Bind studio history to a workspace folder (null = no folder / `__none__`). */
export function setChatHistoryWorkspaceScope(path: string | null): void {
  chatHistoryWorkspaceScope = path
    ? canonicalizeHistoryWorkspacePath(path)
    : null;
}

export function getChatHistoryWorkspaceScope(): string | null {
  return chatHistoryWorkspaceScope;
}

export type LoadedSessions = {
  sessions: SessionMeta[];
  activeId: string | null;
  deletedIds: string[];
};

async function collectSessionIds(
  keyPredicate: (key: string) => boolean,
): Promise<Set<string>> {
  const ids = new Set<string>();
  for (const [key, value] of await store.entries()) {
    if (!keyPredicate(key) || !Array.isArray(value)) continue;
    for (const session of value as SessionMeta[]) {
      if (session?.id) ids.add(session.id);
    }
  }
  return ids;
}

/**
 * One-time split of the flat `sessions:studio` index into per-folder buckets.
 */
async function migrateFlatStudioIndex(
  map: Map<string, unknown>,
): Promise<void> {
  const flatSessions = map.get(FLAT_STUDIO_SESSIONS);
  if (!Array.isArray(flatSessions)) return;

  const flatActive =
    (map.get(FLAT_STUDIO_ACTIVE) as string | null | undefined) ?? null;
  const flatDeleted = map.get(FLAT_STUDIO_DELETED);
  const deletedIds = Array.isArray(flatDeleted)
    ? (flatDeleted as string[])
    : [];

  const buckets = new Map<string, SessionMeta[]>();
  for (const session of flatSessions as SessionMeta[]) {
    const token = session.workspacePath
      ? canonicalizeHistoryWorkspacePath(session.workspacePath)
      : STUDIO_SCOPE_NONE;
    const list = buckets.get(token) ?? [];
    list.push(session);
    buckets.set(token, list);
  }

  for (const [token, sessions] of buckets) {
    const listKey = `sessions:studio:${token}`;
    if (map.has(listKey)) continue;
    const activeId =
      flatActive && sessions.some((session) => session.id === flatActive)
        ? flatActive
        : null;
    await store.set(listKey, sessions);
    await store.set(`activeId:studio:${token}`, activeId);
    await store.set(`deletedSessionIds:studio:${token}`, deletedIds);
    map.set(listKey, sessions);
    map.set(`activeId:studio:${token}`, activeId);
    map.set(`deletedSessionIds:studio:${token}`, deletedIds);
  }

  await store.delete(FLAT_STUDIO_SESSIONS);
  await store.delete(FLAT_STUDIO_ACTIVE);
  await store.delete(FLAT_STUDIO_DELETED);
  map.delete(FLAT_STUDIO_SESSIONS);
  map.delete(FLAT_STUDIO_ACTIVE);
  map.delete(FLAT_STUDIO_DELETED);
}

/**
 * Read the session index for the active surface (+ studio folder scope).
 * Legacy unscoped keys migrate into Desktop (`agent`) only.
 */
export async function loadAll(): Promise<LoadedSessions> {
  const surface = chatHistorySurface;
  const listKey = sessionsListKey(surface);
  const activeKey = activeSessionKey(surface);
  const deletedKey = deletedSessionsKey(surface);

  const entries = await store.entries();
  const map = new Map<string, unknown>(entries);

  if (surface === "studio") {
    await migrateFlatStudioIndex(map);
  }

  const hasNamespaced =
    map.has(listKey) || map.has(activeKey) || map.has(deletedKey);

  if (hasNamespaced) {
    const deletedIds = map.get(deletedKey);
    return {
      sessions: (map.get(listKey) as SessionMeta[] | undefined) ?? [],
      activeId: (map.get(activeKey) as string | null | undefined) ?? null,
      deletedIds: Array.isArray(deletedIds) ? (deletedIds as string[]) : [],
    };
  }

  if (surface === "agent") {
    const legacyDeleted = map.get(LEGACY_KEY_DELETED);
    const migrated: LoadedSessions = {
      sessions: (map.get(LEGACY_KEY_SESSIONS) as SessionMeta[] | undefined) ?? [],
      activeId:
        (map.get(LEGACY_KEY_ACTIVE) as string | null | undefined) ?? null,
      deletedIds: Array.isArray(legacyDeleted)
        ? (legacyDeleted as string[])
        : [],
    };
    await store.set(listKey, migrated.sessions);
    await store.set(activeKey, migrated.activeId);
    await store.set(deletedKey, migrated.deletedIds);
    await store.delete(LEGACY_KEY_SESSIONS);
    await store.delete(LEGACY_KEY_ACTIVE);
    await store.delete(LEGACY_KEY_DELETED);
    return migrated;
  }

  // Studio folder with no prior index — empty, independent of Desktop.
  const empty: LoadedSessions = {
    sessions: [],
    activeId: null,
    deletedIds: [],
  };
  await store.set(listKey, empty.sessions);
  await store.set(activeKey, empty.activeId);
  await store.set(deletedKey, empty.deletedIds);
  return empty;
}

export async function loadMessages(id: string): Promise<UIMessage[] | null> {
  const cached = (await store.get<UIMessage[]>(messagesKey(id))) ?? null;
  if (cached && cached.length > 0) return cached;
  return loadMessagesFromBackend(id);
}

function backendMessageToUi(
  msg: {
    role: string;
    content?: string | null;
    tool_calls?: Array<{
      id: string;
      function: { name: string; arguments: string };
    }> | null;
    tool_call_id?: string | null;
    reasoning_content?: string | null;
  },
  index: number,
): UIMessage {
  const mapped = mapBackendMessageToTranscript(msg, index);
  return {
    id: mapped.id,
    role: mapped.role as UIMessage["role"],
    parts: mapped.parts as UIMessage["parts"],
  };
}

async function loadMessagesFromBackend(id: string): Promise<UIMessage[] | null> {
  try {
    const { native } = await import("./native");
    const sessions =
      (await store.get<SessionMeta[]>(sessionsListKey(chatHistorySurface))) ??
      [];
    const workspacePath =
      sessionWorkspacePathForId(sessions, id) ??
      (chatHistorySurface === "studio"
        ? (chatHistoryWorkspaceScope ?? undefined)
        : undefined);
    const backend = await native.agentGetSessionMessages(id, workspacePath);
    if (!backend || backend.length === 0) return null;
    const ui = backend.map((m, i) => backendMessageToUi(m, i));
    if (ui.length > 0) {
      await store.set(messagesKey(id), ui);
    }
    return ui;
  } catch (cause) {
    console.warn(`Failed to load messages from backend for session ${id}:`, cause);
    return null;
  }
}

export async function saveSessionsList(sessions: SessionMeta[]): Promise<void> {
  await store.set(sessionsListKey(chatHistorySurface), sessions);
}

export async function saveActiveId(id: string | null): Promise<void> {
  await store.set(activeSessionKey(chatHistorySurface), id);
}

export async function saveDeletedIds(ids: string[]): Promise<void> {
  await store.set(deletedSessionsKey(chatHistorySurface), ids);
}

export async function saveMessages(
  id: string,
  messages: UIMessage[],
): Promise<void> {
  await store.set(messagesKey(id), messages);
}

export async function deleteSessionData(id: string): Promise<void> {
  // Keep message bodies if any other surface/folder index still references id.
  const stillReferenced = await collectSessionIds(
    (key) => key.startsWith("sessions:"),
  );
  if (stillReferenced.has(id)) return;
  await store.delete(messagesKey(id));
}

export function newSessionId(): string {
  return newSessionIdShared();
}

export function deriveTitle(messages: UIMessage[]): string {
  return deriveChatTitleFromMessages(messages);
}

/**
 * Merge backend-only sessions into the active index.
 * Studio recovers only from the open folder; agent skips studio-owned ids.
 */
export async function mergeBackendSessions(
  frontend: SessionMeta[],
  deletedIds: string[] = [],
): Promise<{ merged: SessionMeta[]; recoveredIds: string[] }> {
  const surface = chatHistorySurface;
  let backend: { id: string; updatedAt: number; title: string }[] = [];
  try {
    const { native } = await import("./native");
    if (surface === "studio") {
      if (chatHistoryWorkspaceScope) {
        backend = await native.agentListSessions(chatHistoryWorkspaceScope);
      }
    } else {
      const targets = new Set<string | undefined>([undefined]);
      for (const session of frontend) {
        if (session.workspacePath) targets.add(session.workspacePath);
      }
      for (const workspacePath of targets) {
        const items = await native.agentListSessions(workspacePath);
        backend.push(...items);
      }
    }
  } catch {
    return { merged: frontend, recoveredIds: [] };
  }

  const otherIds =
    surface === "agent"
      ? await collectSessionIds((key) => key.startsWith("sessions:studio"))
      : await collectSessionIds((key) => key === "sessions:agent");
  if (otherIds.size > 0) {
    backend = backend.filter((item) => !otherIds.has(item.id));
  }

  const { merged, recoveredIds } = mergeRecoveredSessions(
    frontend,
    backend,
    deletedIds,
  );

  // Stamp studio recoveries with the open folder so they stay in this bucket.
  const scopedMerged =
    surface === "studio" && chatHistoryWorkspaceScope
      ? (merged as SessionMeta[]).map((session) =>
          session.workspacePath
            ? session
            : {
                ...session,
                workspacePath: chatHistoryWorkspaceScope,
                workspaceKind: session.workspaceKind ?? "local",
              },
        )
      : (merged as SessionMeta[]);

  return {
    merged: scopedMerged,
    recoveredIds,
  };
}
