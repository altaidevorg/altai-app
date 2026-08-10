import { describe, expect, it } from "vitest";
import {
  automationCreateSubtitle,
  automationListSubtitle,
} from "../lib/automationListSubtitleChrome.js";

describe("automationListSubtitle", () => {
  it("formats counts", () => {
    expect(automationListSubtitle({ repeat: 2, once: 1 })).toBe(
      "2 recurring · 1 one-time",
    );
  });
});

describe("automationCreateSubtitle", () => {
  it("returns create copy", () => {
    expect(automationCreateSubtitle()).toMatch(/instruction/i);
  });
});
