//! Context-condensing (compaction) configuration shared by all hosts.

use serde::Deserialize;

/// Context-condensing (compaction) configuration received from the JS layer
/// (camelCase IPC) and threaded into the isanagent `AgentLogicParams`. The
/// `auto == false` case is encoded by forcing `threshold_tokens` to
/// `usize::MAX`, which keeps manual `/compact` working while disabling the
/// between-turns auto trigger.
///
/// Field names match the camelCase wire format (`#[serde(rename_all =
/// "camelCase")]` → `thresholdTokens`, `tailTurns`).
#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CompactionArg {
    pub auto: bool,
    pub threshold_tokens: usize,
    pub tail_turns: usize,
}

impl CompactionArg {
    /// Resolve the user-facing compaction knobs into the three values
    /// `AgentLogicParams` actually consumes. `short_term_threshold_turns`
    /// is kept at the isanagent crate default (20) since the public API
    /// doesn't expose a per-call override for it.
    pub fn to_logic_params(&self) -> (usize, usize, usize) {
        // (max_recent_summaries, short_term_threshold_turns, short_term_threshold_tokens)
        let max_recent_summaries = self.tail_turns;
        let short_term_threshold_turns = 20;
        // Floor at 8k so a typo (e.g. 0) can't wedge the loop into compacting
        // every turn; when auto is off, MAX effectively disables the trigger.
        let short_term_threshold_tokens = if self.auto {
            self.threshold_tokens.max(8_000)
        } else {
            usize::MAX
        };
        (
            max_recent_summaries,
            short_term_threshold_turns,
            short_term_threshold_tokens,
        )
    }

    /// Compact tuple used in the runtime fingerprint so a compaction-pref
    /// change rebuilds the instance on next send.
    pub fn fingerprint_tuple(&self) -> (bool, usize, usize) {
        (self.auto, self.threshold_tokens, self.tail_turns)
    }
}

#[cfg(test)]
mod compaction_tests {
    use super::*;

    #[test]
    fn auto_on_passes_threshold_and_tail() {
        let c = CompactionArg {
            auto: true,
            threshold_tokens: 50_000,
            tail_turns: 7,
        };
        let (tail, turns, tokens) = c.to_logic_params();
        assert_eq!(tail, 7);
        assert_eq!(turns, 20); // crate default, not user-configurable
        assert_eq!(tokens, 50_000);
    }

    #[test]
    fn auto_on_floors_threshold_at_8k() {
        // A typo of 0 (or below 8k) must not wedge the loop into compacting
        // every turn.
        let c = CompactionArg {
            auto: true,
            threshold_tokens: 0,
            tail_turns: 5,
        };
        let (_, _, tokens) = c.to_logic_params();
        assert_eq!(tokens, 8_000);
    }

    #[test]
    fn auto_off_disables_via_max_threshold() {
        // auto=false → MAX threshold so the between-turns trigger never fires.
        // Manual /compact still works (it is a direct backend FIFO command).
        let c = CompactionArg {
            auto: false,
            threshold_tokens: 50_000,
            tail_turns: 5,
        };
        let (_, _, tokens) = c.to_logic_params();
        assert_eq!(tokens, usize::MAX);
    }

    #[test]
    fn fingerprint_tuple_round_trips_fields() {
        let c = CompactionArg {
            auto: false,
            threshold_tokens: 12_345,
            tail_turns: 9,
        };
        assert_eq!(c.fingerprint_tuple(), (false, 12_345, 9));
    }
}
