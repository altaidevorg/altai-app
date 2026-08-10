import { describe, expect, it } from "vitest";
import {
  historyToggleLabel,
  miniConversationControlTitle,
} from "../lib/historyToggleChrome.js";

describe("historyToggleChrome", () => {
  it("labels history open vs closed", () => {
    expect(historyToggleLabel(true)).toBe("Back to task");
    expect(historyToggleLabel(false)).toBe("Chat sessions");
  });

  it("labels mini conversation control", () => {
    expect(miniConversationControlTitle(true)).toBe("Mini-window open");
    expect(miniConversationControlTitle(false)).toBe("Open conversation");
  });
});
