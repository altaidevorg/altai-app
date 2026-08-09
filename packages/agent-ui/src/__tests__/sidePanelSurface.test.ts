import { describe, expect, it } from "vitest";
import {
  closeChatTabSelection,
  openIdsAfterNewChat,
  reconcileOpenChatTabIds,
  resolveSidePanelOpenEvent,
  toggleSidePanelChromeSurface,
} from "../lib/sidePanelSurface.js";

describe("sidePanelSurface", () => {
  it("toggles panel surface", () => {
    expect(toggleSidePanelChromeSurface(null, "history")).toBe("history");
    expect(toggleSidePanelChromeSurface("history", "history")).toBeNull();
    expect(toggleSidePanelChromeSurface("history", "inspector")).toBe(
      "inspector",
    );
  });

  it("routes open events", () => {
    expect(resolveSidePanelOpenEvent({ surface: "history" })).toEqual({
      kind: "surface",
      surface: "history",
    });
    expect(resolveSidePanelOpenEvent({ surface: "automations" })).toEqual({
      kind: "operations",
      view: "work",
      workHubView: "scheduled",
    });
    expect(resolveSidePanelOpenEvent({ surface: "review" })).toEqual({
      kind: "review",
    });
  });

  it("reconciles tabs", () => {
    expect(
      reconcileOpenChatTabIds({
        openIds: ["a", "gone"],
        sessionIds: ["a", "b"],
        activeSessionId: "b",
      }),
    ).toEqual(["a", "b"]);
  });

  it("closes tabs with focus rules", () => {
    expect(
      closeChatTabSelection({
        openIds: ["a", "b"],
        closingId: "a",
        activeSessionId: "a",
        createSessionId: () => "x",
      }),
    ).toEqual({ openIds: ["b"], focusSessionId: "b", closedOnly: false });
    expect(openIdsAfterNewChat(["a"], "b")).toEqual(["a", "b"]);
  });
});
