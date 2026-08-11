import { createElement } from "react";
import { describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { WorkList } from "../components/WorkList.js";
import { WorkDetail } from "../components/WorkDetail.js";
import { WorkInbox } from "../components/WorkInbox.js";
import { NewWorkDialog } from "../components/NewWorkDialog.js";

describe("WorkList", () => {
  it("renders filters and New Work", () => {
    const html = renderToStaticMarkup(
      createElement(WorkList, {
        status: "ready",
        filter: "my_active",
        onFilterChange: () => {},
        rows: [
          {
            id: "w1",
            title: "Ship Work list",
            projectLabel: "altai",
            stateLabel: "ready",
            attemptLabel: "idle",
            updatedLabel: "1m",
          },
        ],
        onOpenWork: () => {},
        onNewWork: () => {},
      }),
    );
    expect(html).toContain("New Work");
    expect(html).toContain("My active");
    expect(html).toContain("Ship Work list");
  });

  it("renders empty copy for my active", () => {
    const html = renderToStaticMarkup(
      createElement(WorkList, {
        status: "ready",
        filter: "my_active",
        onFilterChange: () => {},
        rows: [],
        onOpenWork: () => {},
        onNewWork: () => {},
        onOpenInbox: () => {},
      }),
    );
    expect(html).toContain("Nothing active");
    expect(html).toContain("Inbox");
  });
});

describe("WorkDetail", () => {
  it("renders sticky accept/return actions", () => {
    const html = renderToStaticMarkup(
      createElement(WorkDetail, {
        status: "ready",
        title: "Ship Work list",
        stateLabel: "in review",
        primaryActions: ["accept", "return"],
        onPrimaryAction: () => {},
        attempts: [
          {
            id: "a1",
            label: "#1",
            phaseLabel: "succeeded",
            onOpenRun: () => {},
          },
        ],
      }),
    );
    expect(html).toContain("Accept");
    expect(html).toContain("Return");
    expect(html).toContain("Open run");
  });
});

describe("WorkInbox", () => {
  it("renders empty state", () => {
    const html = renderToStaticMarkup(
      createElement(WorkInbox, {
        status: "ready",
        rows: [],
        onOpenWork: () => {},
        onGoToWork: () => {},
      }),
    );
    expect(html).toContain("Nothing needs you");
    expect(html).toContain("Go to Work");
  });

  it("renders actionable rows", () => {
    const html = renderToStaticMarkup(
      createElement(WorkInbox, {
        status: "ready",
        rows: [
          {
            id: "i1",
            workId: "w1",
            kind: "review_required",
            title: "Ship Work list",
            why: "Attempt finished — decide Accept or Return",
            ageLabel: "2m",
          },
        ],
        onOpenWork: () => {},
      }),
    );
    expect(html).toContain("Review");
    expect(html).toContain("Ship Work list");
  });
});

describe("NewWorkDialog", () => {
  it("renders when open", () => {
    const onCreate = vi.fn();
    const html = renderToStaticMarkup(
      createElement(NewWorkDialog, {
        open: true,
        projectLabel: "altai",
        onClose: () => {},
        onCreate,
      }),
    );
    expect(html).toContain("New Work");
    expect(html).toContain("Acceptance criteria");
    expect(html).toContain("Create");
  });

  it("renders nothing when closed", () => {
    const html = renderToStaticMarkup(
      createElement(NewWorkDialog, {
        open: false,
        projectLabel: "altai",
        onClose: () => {},
        onCreate: () => {},
      }),
    );
    expect(html).toBe("");
  });
});
