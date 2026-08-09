import { describe, expect, it } from "vitest";
import {
  isEscapeDismissKey,
  shouldDismissSidePanelOnEscape,
  isTextEditingKeyboardTarget,
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


describe("isTextEditingKeyboardTarget", () => {
  it("matches input/textarea/select and contenteditable", () => {
    expect(isTextEditingKeyboardTarget({ tagName: "INPUT" })).toBe(true);
    expect(isTextEditingKeyboardTarget({ tagName: "DIV" })).toBe(false);
    expect(isTextEditingKeyboardTarget({ isContentEditable: true })).toBe(true);
  });
});
