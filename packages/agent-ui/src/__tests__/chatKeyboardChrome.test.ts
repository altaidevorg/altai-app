import { describe, expect, it } from "vitest";
import {
  isEscapeDismissKey,
  shouldDismissSidePanelOnEscape,
} from "../lib/chatKeyboardChrome.js";

describe("isEscapeDismissKey", () => {
  it("matches plain Escape", () => {
    expect(isEscapeDismissKey({ key: "Escape" })).toBe(true);
    expect(isEscapeDismissKey({ key: "Escape", ctrlKey: true })).toBe(false);
    expect(isEscapeDismissKey({ key: "Enter" })).toBe(false);
  });
});


describe("shouldDismissSidePanelOnEscape", () => {
  it("closes only plain escape outside fields/overlays", () => {
    expect(shouldDismissSidePanelOnEscape({ key: "Escape" })).toBe(true);
    expect(
      shouldDismissSidePanelOnEscape({ key: "Escape", isEditableTarget: true }),
    ).toBe(false);
    expect(
      shouldDismissSidePanelOnEscape({ key: "Escape", hasOpenOverlay: true }),
    ).toBe(false);
  });
});
