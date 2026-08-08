import { describe, expect, it } from "vitest";
import {
  canMountCheckpointChrome,
  canRestoreCheckpoint,
  preferredCheckpointLabel,
  toCheckpointMenuItems,
} from "../lib/checkpointChrome.js";

describe("canMountCheckpointChrome", () => {
  it("requires list capability and an active chat", () => {
    expect(
      canMountCheckpointChrome({
        canList: true,
        canRestore: true,
        hasActiveChat: true,
      }),
    ).toBe(true);
    expect(
      canMountCheckpointChrome({
        canList: false,
        canRestore: true,
        hasActiveChat: true,
      }),
    ).toBe(false);
    expect(
      canMountCheckpointChrome({
        canList: true,
        canRestore: false,
        hasActiveChat: true,
      }),
    ).toBe(true);
    expect(
      canMountCheckpointChrome({
        canList: true,
        canRestore: true,
        hasActiveChat: false,
      }),
    ).toBe(false);
  });
});

describe("canRestoreCheckpoint", () => {
  it("blocks while another restore is in flight", () => {
    expect(
      canRestoreCheckpoint(
        { canList: true, canRestore: true, hasActiveChat: true },
        "cp1",
      ),
    ).toBe(false);
    expect(
      canRestoreCheckpoint(
        { canList: true, canRestore: true, hasActiveChat: true },
        null,
      ),
    ).toBe(true);
  });
});

describe("preferredCheckpointLabel", () => {
  it("prefers path and pairs with tool label", () => {
    expect(
      preferredCheckpointLabel({
        path: "/ws/a.ts",
        label: "edit_file",
      }),
    ).toBe("/ws/a.ts · edit_file");
    expect(preferredCheckpointLabel({ path: "/ws/a.ts" })).toBe("/ws/a.ts");
    expect(preferredCheckpointLabel({ label: "edit_file" })).toBe("edit_file");
  });
});

describe("toCheckpointMenuItems", () => {
  it("maps host rows for the shared panel", () => {
    const items = toCheckpointMenuItems([
      {
        id: "c1",
        chatId: "chat",
        createdAt: "2023-11-14T22:13:20.000Z",
        label: "/ws/foo.ts · edit_file",
      },
    ]);
    expect(items).toEqual([
      {
        id: "c1",
        path: "/ws/foo.ts · edit_file",
        label: "/ws/foo.ts · edit_file",
        createdMs: Date.parse("2023-11-14T22:13:20.000Z"),
      },
    ]);
  });
});
