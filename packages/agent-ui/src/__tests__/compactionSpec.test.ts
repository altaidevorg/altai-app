import { describe, expect, it } from "vitest";
import { resolveCompactionSpecFromContext } from "../lib/compactionSpec.js";

describe("resolveCompactionSpecFromContext", () => {
  const base = {
    compactionAuto: true,
    compactionThresholdPercent: null as number | null,
    compactionThresholdTokens: 40_000,
    compactionTailTurns: 4,
  };

  it("uses absolute tokens when percent is null", () => {
    expect(resolveCompactionSpecFromContext(base, 200_000)).toEqual({
      auto: true,
      thresholdTokens: 40_000,
      tailTurns: 4,
    });
  });

  it("converts percent against context limit", () => {
    expect(
      resolveCompactionSpecFromContext(
        { ...base, compactionThresholdPercent: 50 },
        100_000,
      ),
    ).toEqual({
      auto: true,
      thresholdTokens: 50_000,
      tailTurns: 4,
    });
  });

  it("preserves auto off", () => {
    expect(
      resolveCompactionSpecFromContext({ ...base, compactionAuto: false }, 1),
    ).toMatchObject({ auto: false });
  });
});
