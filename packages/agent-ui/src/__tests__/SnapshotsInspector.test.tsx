import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { SnapshotsInspector } from "../components/SnapshotsInspector.js";

describe("SnapshotsInspector", () => {
  it("renders empty state when there are no snapshots", () => {
    const html = renderToStaticMarkup(
      createElement(SnapshotsInspector, {
        applied: [],
        items: [],
        restoringId: null,
        error: null,
        onRestoreApplied: () => {},
        onRestoreCheckpoint: () => {},
      }),
    );
    expect(html).toContain("ready to restore safely");
  });

  it("renders plan review and agent edit sections", () => {
    const html = renderToStaticMarkup(
      createElement(SnapshotsInspector, {
        applied: [
          { id: "p1", path: "src/new.ts", isNewFile: true },
          { id: "p2", path: "src/old.ts", isNewFile: false },
        ],
        items: [{ id: "c1", path: "src/edit.ts", label: "Before edit" }],
        restoringId: "c1",
        error: "Could not restore change.",
        onRestoreApplied: () => {},
        onRestoreCheckpoint: () => {},
      }),
    );
    expect(html).toContain("Plan review");
    expect(html).toContain("Agent edits");
    // newest applied first
    expect(html.indexOf("old.ts")).toBeLessThan(html.indexOf("new.ts"));
    expect(html).toContain("removes new file");
    expect(html).toContain("restores prior content");
    expect(html).toContain("Before edit");
    expect(html).toContain("Restoring…");
    expect(html).toContain("Could not restore change.");
  });
});
