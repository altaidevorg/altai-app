import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import {
  CheckpointMenuPanel,
  checkpointBasename,
  formatCheckpointTimeAgo,
} from "../components/CheckpointMenuPanel.js";

describe("CheckpointMenuPanel helpers", () => {
  it("basenames posix and windows paths", () => {
    expect(checkpointBasename("/tmp/foo/bar.ts")).toBe("bar.ts");
    expect(checkpointBasename("C:\\proj\\src\\main.rs")).toBe("main.rs");
    expect(checkpointBasename("alone")).toBe("alone");
  });

  it("formats relative ages", () => {
    const now = 1_700_000_000_000;
    expect(formatCheckpointTimeAgo(now - 10_000, now)).toBe("just now");
    expect(formatCheckpointTimeAgo(now - 5 * 60_000, now)).toBe("5m ago");
    expect(formatCheckpointTimeAgo(now - 3 * 3_600_000, now)).toBe("3h ago");
    expect(formatCheckpointTimeAgo(now - 2 * 86_400_000, now)).toBe("2d ago");
  });
});

describe("CheckpointMenuPanel", () => {
  it("renders empty state", () => {
    const html = renderToStaticMarkup(
      createElement(CheckpointMenuPanel, {
        items: [],
        onRestore: () => {},
      }),
    );
    expect(html).toContain("Edit checkpoints");
    expect(html).toContain("No checkpoints yet");
  });

  it("renders items with restore affordance", () => {
    const now = 1_700_000_000_000;
    const html = renderToStaticMarkup(
      createElement(CheckpointMenuPanel, {
        items: [
          {
            id: "cp-1",
            path: "/ws/src/App.tsx",
            label: "before edit",
            createdMs: now - 120_000,
          },
        ],
        restoringId: "cp-1",
        onRestore: () => {},
        nowMs: now,
      }),
    );
    expect(html).toContain("App.tsx");
    expect(html).toContain("before edit");
    expect(html).toContain("2m ago");
    expect(html).toContain("Restoring…");
  });
});
