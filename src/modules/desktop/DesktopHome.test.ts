import { describe, expect, it } from "vitest";
import { toHomeInboxRow } from "./DesktopHome";

describe("DesktopHome projections", () => {
  it("maps canonical Inbox attention into an actionable row", () => {
    const row = toHomeInboxRow({
      id: "inbox_1",
      workId: "work_1",
      kind: "review_required",
      title: "Consolidate Desktop shell",
      why: "Attempt finished",
      createdAtMs: Date.now() - 5_000,
    });

    expect(row).toMatchObject({
      id: "inbox_1",
      workId: "work_1",
      kind: "review_required",
      why: "Attempt finished",
    });
  });
});
