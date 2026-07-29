//! Compaction preference resolution for ALTAI CLI / Desktop adapters.

use std::env;

/// User-facing compaction knobs (mirrors desktop `CompactionArg`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompactionPrefs {
    pub auto: bool,
    pub threshold_tokens: usize,
    pub tail_turns: usize,
}

impl Default for CompactionPrefs {
    fn default() -> Self {
        Self {
            auto: true,
            threshold_tokens: 100_000,
            tail_turns: 5,
        }
    }
}

/// Values consumed by IsanAgent `AgentLogicParams`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompactionLogicParams {
    pub max_recent_summaries: usize,
    pub short_term_threshold_turns: usize,
    pub short_term_threshold_tokens: usize,
}

impl CompactionPrefs {
    /// Resolve into IsanAgent agent-logic parameters.
    ///
    /// When `auto` is false, the token threshold becomes `usize::MAX` so
    /// between-turn auto-compaction never fires while manual `/compact` still works.
    pub fn to_logic_params(self) -> CompactionLogicParams {
        CompactionLogicParams {
            max_recent_summaries: self.tail_turns.max(1),
            short_term_threshold_turns: 20,
            short_term_threshold_tokens: if self.auto {
                self.threshold_tokens.max(8_000)
            } else {
                usize::MAX
            },
        }
    }
}

/// CLI / env overrides layered onto defaults (and later config-file layers).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CompactionOverrides {
    pub auto: Option<bool>,
    pub threshold_tokens: Option<usize>,
    pub tail_turns: Option<usize>,
}

/// Resolve compaction prefs.
///
/// Precedence: explicit overrides → `ALTAI_DISABLE_AUTOCOMPACT` / `ALTAI_COMPACT_*`
/// env → defaults. Desktop settings JSON bridging can feed `overrides` from the
/// CLI after reading `altai-settings.json`.
pub fn resolve_compaction_prefs(overrides: CompactionOverrides) -> CompactionPrefs {
    let mut prefs = CompactionPrefs::default();

    if env_truthy("ALTAI_DISABLE_AUTOCOMPACT") {
        prefs.auto = false;
    }
    if let Ok(raw) = env::var("ALTAI_COMPACT_THRESHOLD") {
        if let Ok(n) = raw.trim().parse::<usize>() {
            prefs.threshold_tokens = n;
        }
    }
    if let Ok(raw) = env::var("ALTAI_COMPACT_TAIL") {
        if let Ok(n) = raw.trim().parse::<usize>() {
            prefs.tail_turns = n;
        }
    }

    if let Some(auto) = overrides.auto {
        prefs.auto = auto;
    }
    if let Some(tokens) = overrides.threshold_tokens {
        prefs.threshold_tokens = tokens;
    }
    if let Some(tail) = overrides.tail_turns {
        prefs.tail_turns = tail;
    }

    prefs
}

fn env_truthy(name: &str) -> bool {
    matches!(
        env::var(name).as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes") | Ok("on")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_off_disables_token_trigger() {
        let prefs = CompactionPrefs {
            auto: false,
            threshold_tokens: 50_000,
            tail_turns: 7,
        };
        let logic = prefs.to_logic_params();
        assert_eq!(logic.max_recent_summaries, 7);
        assert_eq!(logic.short_term_threshold_tokens, usize::MAX);
    }

    #[test]
    fn auto_on_floors_threshold_at_8k() {
        let prefs = CompactionPrefs {
            auto: true,
            threshold_tokens: 100,
            tail_turns: 3,
        };
        assert_eq!(prefs.to_logic_params().short_term_threshold_tokens, 8_000);
    }

    #[test]
    fn overrides_win_over_defaults() {
        let prefs = resolve_compaction_prefs(CompactionOverrides {
            auto: Some(false),
            threshold_tokens: Some(42_000),
            tail_turns: Some(9),
        });
        assert!(!prefs.auto);
        assert_eq!(prefs.threshold_tokens, 42_000);
        assert_eq!(prefs.tail_turns, 9);
    }
}
