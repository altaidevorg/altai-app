/**
 * Pure compaction threshold resolve (A6.159).
 * Host supplies model context limit tokens; package does percent→token math.
 */

export type CompactionPrefs = {
  compactionAuto: boolean;
  compactionThresholdPercent: number | null;
  compactionThresholdTokens: number;
  compactionTailTurns: number;
};

export type CompactionSpec = {
  auto: boolean;
  thresholdTokens: number;
  tailTurns: number;
};

/**
 * Resolve user-facing compaction prefs into the runtime `(auto, thresholdTokens,
 * tailTurns)` tuple. When a percent threshold is set, convert against the
 * provided model context window; otherwise use absolute tokens.
 */
export function resolveCompactionSpecFromContext(
  prefs: CompactionPrefs,
  contextLimitTokens: number,
): CompactionSpec {
  const thresholdTokens =
    prefs.compactionThresholdPercent != null
      ? Math.round(
          (prefs.compactionThresholdPercent / 100) * contextLimitTokens,
        )
      : prefs.compactionThresholdTokens;
  return {
    auto: prefs.compactionAuto,
    thresholdTokens,
    tailTurns: prefs.compactionTailTurns,
  };
}
