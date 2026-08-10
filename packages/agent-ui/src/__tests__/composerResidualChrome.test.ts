import { describe, expect, it } from "vitest";
import {
  COMPOSER_ATTACH_DIFF_ERROR_LABEL,
  COMPOSER_PERMISSION_MODE_LABEL,
} from "../lib/composerResidualChrome.js";

describe("composerResidualChrome", () => {
  it("exposes residual composer labels", () => {
    expect(COMPOSER_ATTACH_DIFF_ERROR_LABEL).toContain("diff");
    expect(COMPOSER_PERMISSION_MODE_LABEL).toBe("Permission mode");
  });
});
