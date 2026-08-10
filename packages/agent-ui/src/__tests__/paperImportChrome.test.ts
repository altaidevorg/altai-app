import { describe, expect, it } from "vitest";
import {
  COMPACT_NOW_TITLE,
  PAPER_URL_ARIA_LABEL,
  paperImportSubmitLabel,
} from "../lib/paperImportChrome.js";

describe("paperImportChrome", () => {
  it("exposes paper import and compact labels", () => {
    expect(PAPER_URL_ARIA_LABEL).toBe("Paper URL");
    expect(paperImportSubmitLabel(true)).toBe("Fetching...");
    expect(paperImportSubmitLabel(false)).toBe("Fetch");
    expect(COMPACT_NOW_TITLE).toContain("Compact context");
  });
});
