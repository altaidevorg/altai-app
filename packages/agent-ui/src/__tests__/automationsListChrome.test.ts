import { describe, expect, it } from "vitest";
import {
  AUTOMATIONS_EMPTY_TITLE,
  AUTOMATIONS_FILTERED_EMPTY_MESSAGE,
  AUTOMATIONS_OWNING_CHAT_FALLBACK,
  AUTOMATIONS_SELECT_CHAT_LABEL,
} from "../lib/automationsListChrome.js";

describe("automationsListChrome", () => {
  it("exposes empty and list chrome copy", () => {
    expect(AUTOMATIONS_EMPTY_TITLE).toContain("schedules");
    expect(AUTOMATIONS_FILTERED_EMPTY_MESSAGE).toContain("No automations");
    expect(AUTOMATIONS_OWNING_CHAT_FALLBACK).toBe("Owning chat");
    expect(AUTOMATIONS_SELECT_CHAT_LABEL).toBe("Select a chat");
  });
});
