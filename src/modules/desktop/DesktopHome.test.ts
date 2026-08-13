import { describe, expect, it } from "vitest";
import { toHomeInboxRow, toHomeWorkRow } from "./DesktopHome";

describe("DesktopHome projections", () => {
  it("maps canonical Work into a Home row", () => {
    const row = toHomeWorkRow(
      {
        id: "work_1",
        projectId: "project_1",
        title: "Consolidate Desktop shell",
        description: "",
        acceptanceCriteria: "",
        kind: "task",
        state: "in_progress",
        revision: 1,
        createdAtMs: Date.now() - 10_000,
        updatedAtMs: Date.now() - 5_000,
      },
      "altai-app",
    );

    expect(row).toMatchObject({
      id: "work_1",
      title: "Consolidate Desktop shell",
      projectLabel: "altai-app",
      stateLabel: "in progress",
    });
  });

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
