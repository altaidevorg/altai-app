import { createElement, createRef } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { ChatHistoryPanel } from "../components/ChatHistoryPanel.js";
import {
  groupSessionsByRecency,
  sessionHistoryBucket,
  startOfDay,
} from "../lib/sessionHistory.js";

describe("sessionHistory helpers", () => {
  it("buckets by recency", () => {
    const now = new Date(2026, 7, 4, 15, 0, 0).getTime();
    const nowDay = startOfDay(now);
    expect(sessionHistoryBucket(now, nowDay)).toBe("Today");
    expect(sessionHistoryBucket(nowDay - 24 * 60 * 60 * 1000, nowDay)).toBe(
      "Yesterday",
    );
    expect(
      sessionHistoryBucket(nowDay - 3 * 24 * 60 * 60 * 1000, nowDay),
    ).toBe("Previous 7 days");
  });

  it("groups and sorts newest first", () => {
    const now = new Date(2026, 7, 4, 12, 0, 0).getTime();
    const groups = groupSessionsByRecency(
      [
        { id: "a", title: "Older today", updatedAt: now - 3_600_000 },
        { id: "b", title: "Newer today", updatedAt: now - 60_000 },
        {
          id: "c",
          title: "Yesterday chat",
          updatedAt: startOfDay(now) - 12 * 60 * 60 * 1000,
        },
      ],
      now,
    );
    expect(groups.map((g) => g.label)).toEqual(["Today", "Yesterday"]);
    expect(groups[0]?.items.map((i) => i.id)).toEqual(["b", "a"]);
  });
});

describe("ChatHistoryPanel", () => {
  it("renders empty and new-chat affordances", () => {
    const html = renderToStaticMarkup(
      createElement(ChatHistoryPanel, {
        groups: [],
        activeId: null,
        search: "",
        onSearchChange: () => {},
        onNewChat: () => {},
        onPick: () => {},
        onDelete: () => {},
        renamingId: null,
        renameValue: "",
        onStartRename: () => {},
        onCommitRename: () => {},
        onCancelRename: () => {},
        onRenameValueChange: () => {},
        renameInputRef: createRef<HTMLInputElement>(),
      }),
    );
    expect(html).toContain("New chat");
    expect(html).toContain("Search chat history");
    expect(html).toContain("No chats yet.");
  });

  it("renders grouped sessions and search-empty copy", () => {
    const html = renderToStaticMarkup(
      createElement(ChatHistoryPanel, {
        groups: [
          {
            label: "Today",
            items: [
              {
                id: "s1",
                title: "Refactor panel",
                updatedAt: 1,
                snippet: "Extract shared UI",
              },
            ],
          },
        ],
        activeId: "s1",
        search: "zzz",
        onSearchChange: () => {},
        onNewChat: () => {},
        onPick: () => {},
        onDelete: () => {},
        renamingId: null,
        renameValue: "",
        onStartRename: () => {},
        onCommitRename: () => {},
        onCancelRename: () => {},
        onRenameValueChange: () => {},
        renameInputRef: createRef<HTMLInputElement>(),
      }),
    );
    expect(html).toContain("Today");
    expect(html).toContain("Refactor panel");
    expect(html).toContain("Extract shared UI");
  });

  it("shows no-match empty when search has no groups", () => {
    const html = renderToStaticMarkup(
      createElement(ChatHistoryPanel, {
        groups: [],
        activeId: null,
        search: "missing",
        onSearchChange: () => {},
        onNewChat: () => {},
        onPick: () => {},
        onDelete: () => {},
        renamingId: null,
        renameValue: "",
        onStartRename: () => {},
        onCommitRename: () => {},
        onCancelRename: () => {},
        onRenameValueChange: () => {},
        renameInputRef: createRef<HTMLInputElement>(),
      }),
    );
    expect(html).toContain("No chats match.");
  });
});
