import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("./native", () => ({
  native: {
    agentListSessions: vi.fn(async () => []),
    agentGetSessionMessages: vi.fn(async () => []),
  },
}));

import {
  activeSessionKey,
  deletedSessionsKey,
  getChatHistorySurface,
  loadAll,
  saveActiveId,
  saveSessionsList,
  sessionsListKey,
  setChatHistorySurface,
  setChatHistoryWorkspaceScope,
  type SessionMeta,
} from "./sessions";
import { createAppStore } from "@/lib/appStore";

const STORE_PATH = "altai-ai-sessions.json";

function sampleSession(
  id: string,
  title: string,
  workspacePath?: string | null,
): SessionMeta {
  return {
    id,
    title,
    createdAt: 1,
    updatedAt: 1,
    ...(workspacePath !== undefined ? { workspacePath } : {}),
  };
}

describe("chat history surface partitioning", () => {
  beforeEach(async () => {
    setChatHistorySurface("agent");
    setChatHistoryWorkspaceScope(null);
    const store = createAppStore(STORE_PATH);
    for (const [key] of await store.entries()) {
      await store.delete(key);
    }
  });

  afterEach(() => {
    setChatHistorySurface("agent");
    setChatHistoryWorkspaceScope(null);
  });

  it("keeps Desktop and Workspace session lists independent", async () => {
    setChatHistorySurface("agent");
    await saveSessionsList([sampleSession("desktop-1", "Desktop chat")]);
    await saveActiveId("desktop-1");

    setChatHistorySurface("studio");
    setChatHistoryWorkspaceScope("/Users/me/proj-a");
    await saveSessionsList([sampleSession("ide-1", "IDE chat", "/Users/me/proj-a")]);
    await saveActiveId("ide-1");

    setChatHistorySurface("agent");
    setChatHistoryWorkspaceScope(null);
    const desktop = await loadAll();
    expect(desktop.sessions.map((s) => s.id)).toEqual(["desktop-1"]);
    expect(desktop.activeId).toBe("desktop-1");

    setChatHistorySurface("studio");
    setChatHistoryWorkspaceScope("/Users/me/proj-a");
    const ide = await loadAll();
    expect(ide.sessions.map((s) => s.id)).toEqual(["ide-1"]);
    expect(ide.activeId).toBe("ide-1");
  });

  it("keeps Desktop IDE history separate per workspace folder", async () => {
    setChatHistorySurface("studio");
    setChatHistoryWorkspaceScope("/Users/me/proj-a");
    await saveSessionsList([sampleSession("a1", "A chat", "/Users/me/proj-a")]);
    await saveActiveId("a1");

    setChatHistoryWorkspaceScope("/Users/me/proj-b");
    await saveSessionsList([sampleSession("b1", "B chat", "/Users/me/proj-b")]);
    await saveActiveId("b1");

    setChatHistoryWorkspaceScope("/Users/me/proj-a");
    const a = await loadAll();
    expect(a.sessions.map((s) => s.id)).toEqual(["a1"]);
    expect(a.activeId).toBe("a1");

    setChatHistoryWorkspaceScope("/Users/me/proj-b");
    const b = await loadAll();
    expect(b.sessions.map((s) => s.id)).toEqual(["b1"]);
    expect(b.activeId).toBe("b1");

    const store = createAppStore(STORE_PATH);
    expect(await store.get(sessionsListKey("studio", "/Users/me/proj-a"))).toEqual(
      a.sessions,
    );
    expect(await store.get(sessionsListKey("studio", "/Users/me/proj-b"))).toEqual(
      b.sessions,
    );
  });

  it("migrates legacy unscoped keys into Desktop only", async () => {
    const store = createAppStore(STORE_PATH);
    await store.set("sessions", [sampleSession("legacy-1", "Old chat")]);
    await store.set("activeId", "legacy-1");
    await store.set("deletedSessionIds", ["gone"]);

    setChatHistorySurface("studio");
    setChatHistoryWorkspaceScope("/Users/me/proj");
    const studio = await loadAll();
    expect(studio.sessions).toEqual([]);
    expect(studio.activeId).toBeNull();

    setChatHistorySurface("agent");
    setChatHistoryWorkspaceScope(null);
    const agent = await loadAll();
    expect(agent.sessions.map((s) => s.id)).toEqual(["legacy-1"]);
    expect(agent.activeId).toBe("legacy-1");
    expect(agent.deletedIds).toEqual(["gone"]);

    expect(await store.get("sessions")).toBeUndefined();
    expect(await store.get(sessionsListKey("agent"))).toEqual(agent.sessions);
    expect(await store.get(activeSessionKey("agent"))).toBe("legacy-1");
    expect(await store.get(deletedSessionsKey("agent"))).toEqual(["gone"]);
  });

  it("migrates flat studio index into per-folder buckets", async () => {
    const store = createAppStore(STORE_PATH);
    await store.set("sessions:studio", [
      sampleSession("a1", "A", "/Users/me/proj-a"),
      sampleSession("b1", "B", "/Users/me/proj-b"),
      sampleSession("n1", "None"),
    ]);
    await store.set("activeId:studio", "a1");
    await store.set("deletedSessionIds:studio", ["x"]);

    setChatHistorySurface("studio");
    setChatHistoryWorkspaceScope("/Users/me/proj-a");
    const a = await loadAll();
    expect(a.sessions.map((s) => s.id)).toEqual(["a1"]);
    expect(a.activeId).toBe("a1");
    expect(a.deletedIds).toEqual(["x"]);

    setChatHistoryWorkspaceScope("/Users/me/proj-b");
    const b = await loadAll();
    expect(b.sessions.map((s) => s.id)).toEqual(["b1"]);
    expect(b.activeId).toBeNull();

    setChatHistoryWorkspaceScope(null);
    const none = await loadAll();
    expect(none.sessions.map((s) => s.id)).toEqual(["n1"]);

    expect(await store.get("sessions:studio")).toBeUndefined();
  });

  it("tracks the active surface setter", () => {
    setChatHistorySurface("studio");
    expect(getChatHistorySurface()).toBe("studio");
    setChatHistorySurface("agent");
    expect(getChatHistorySurface()).toBe("agent");
  });
});
