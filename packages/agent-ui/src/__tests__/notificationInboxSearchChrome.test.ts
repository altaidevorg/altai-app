import { describe, expect, it } from "vitest";
import {
  filterRowsBySearchFields,
  isNotificationInboxEmpty,
} from "../lib/notificationInboxSearchChrome.js";

describe("isNotificationInboxEmpty", () => {
  it("is empty only when all zero", () => {
    expect(
      isNotificationInboxEmpty({
        waitingTickets: 0,
        notifications: 0,
        waitingJobs: 0,
      }),
    ).toBe(true);
    expect(
      isNotificationInboxEmpty({
        waitingTickets: 1,
        notifications: 0,
        waitingJobs: 0,
      }),
    ).toBe(false);
  });
});

describe("filterRowsBySearchFields", () => {
  it("filters with host field picker", () => {
    const rows = [
      { id: "1", title: "Ship" },
      { id: "2", title: "Docs" },
    ];
    expect(
      filterRowsBySearchFields(rows, "doc", (row) => [row.title]).map(
        (r) => r.id,
      ),
    ).toEqual(["2"]);
    expect(
      filterRowsBySearchFields(rows, "  ", (row) => [row.title]).map(
        (r) => r.id,
      ),
    ).toEqual(["1", "2"]);
  });
});
