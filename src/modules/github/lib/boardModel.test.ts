import { describe, expect, it } from "vitest";
import {
  BOARD_COLUMNS,
  issueToBoardItem,
  pullToBoardItem,
} from "./boardModel";
import type { GHItem } from "./items";

function item(overrides: Partial<GHItem> = {}): GHItem {
  return {
    number: 12,
    title: "Test work",
    body: "Details",
    html_url: "https://github.com/altai/test/issues/12",
    state: "open",
    user: { login: "efecnc", avatar_url: "" },
    labels: [],
    comments: 0,
    created_at: "2026-01-01T00:00:00Z",
    updated_at: "2026-01-01T00:00:00Z",
    ...overrides,
  };
}

describe("boardModel", () => {
  it("includes a dedicated review column", () => {
    expect(BOARD_COLUMNS.map((column) => column.id)).toEqual([
      "todo",
      "in_progress",
      "review",
      "done",
    ]);
  });

  it("places open pull requests in review", () => {
    expect(pullToBoardItem(item()).status).toBe("review");
  });

  it("keeps open issues in todo and closed issues in done", () => {
    expect(issueToBoardItem(item()).status).toBe("todo");
    expect(issueToBoardItem(item({ state: "closed" })).status).toBe("done");
  });
});
