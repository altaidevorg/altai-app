import { describe, expect, it } from "vitest";
import {
  matchesSearchFields,
  notificationInboxFilterCounts,
  notificationInboxHasVisibleItems,
  notificationsForInboxFilter,
  partitionNotificationsByReadState,
} from "../lib/notificationInboxFilterChrome.js";

describe("matchesSearchFields", () => {
  it("matches normalized haystack", () => {
    expect(matchesSearchFields(["Hello", null, "World"], "hello")).toBe(true);
    expect(matchesSearchFields(["Hello", null, "World"], "world")).toBe(true);
    expect(matchesSearchFields(["Hello"], "zzz")).toBe(false);
  });
});

describe("notificationInboxFilterCounts", () => {
  it("builds tab counts", () => {
    expect(
      notificationInboxFilterCounts({
        waitingTickets: 1,
        notifications: 3,
        waitingJobs: 2,
        unreadNotifications: 1,
      }),
    ).toEqual({ all: 6, attention: 4, updates: 3 });
  });
});

describe("notificationInboxHasVisibleItems", () => {
  it("gates buckets by filter", () => {
    expect(
      notificationInboxHasVisibleItems("updates", {
        tickets: 1,
        notifications: 0,
        waitingJobs: 1,
      }),
    ).toBe(false);
    expect(
      notificationInboxHasVisibleItems("updates", {
        tickets: 0,
        notifications: 1,
        waitingJobs: 0,
      }),
    ).toBe(true);
  });
});

describe("notificationsForInboxFilter + partition", () => {
  const all = [
    { id: "a", seenAtMs: null as number | null },
    { id: "b", seenAtMs: 1 },
  ];
  const unread = [all[0]];
  it("attention uses unread list", () => {
    expect(notificationsForInboxFilter("attention", all, unread)).toEqual(
      unread,
    );
    expect(notificationsForInboxFilter("all", all, unread)).toEqual(all);
  });
  it("partitions read state", () => {
    expect(partitionNotificationsByReadState(all)).toEqual({
      unread: [all[0]],
      read: [all[1]],
    });
  });
});
