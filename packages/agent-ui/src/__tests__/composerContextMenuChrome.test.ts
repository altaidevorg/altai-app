import { describe, expect, it } from "vitest";
import {
  COMPOSER_ATTACH_FILE_TITLE,
  COMPOSER_CONTEXT_ACTIVE_FILE,
  COMPOSER_CONTEXT_WORKING_DIFF,
  COMPOSER_RESEARCH_SEMBLE_TITLE,
} from "../lib/composerContextMenuChrome.js";

describe("composerContextMenuChrome", () => {
  it("exposes attach and context action copy", () => {
    expect(COMPOSER_ATTACH_FILE_TITLE).toContain("Attach");
    expect(COMPOSER_CONTEXT_ACTIVE_FILE.label).toBe("Active file");
    expect(COMPOSER_CONTEXT_WORKING_DIFF.detail).toContain("Git");
    expect(COMPOSER_RESEARCH_SEMBLE_TITLE).toContain("Semble");
  });
});
