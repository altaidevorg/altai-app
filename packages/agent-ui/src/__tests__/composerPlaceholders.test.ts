import { describe, expect, it } from "vitest";
import {
  COMPOSER_PLACEHOLDERS,
  pickPlaceholder,
} from "../lib/composerPlaceholders.js";

describe("pickPlaceholder", () => {
  it("selects from catalog deterministically", () => {
    expect(pickPlaceholder(() => 0)).toBe(COMPOSER_PLACEHOLDERS[0]);
    expect(pickPlaceholder(() => 0.999)).toBe(
      COMPOSER_PLACEHOLDERS[COMPOSER_PLACEHOLDERS.length - 1],
    );
  });
});
