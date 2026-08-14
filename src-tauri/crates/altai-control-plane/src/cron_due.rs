//! Pure cron due-evaluation for routine materialization (package 041). Given a
//! cron expression and an anchor timestamp (a routine's last-fired time, or its
//! current revision's creation time when it has never fired), the next scheduled
//! fire is the first cron match strictly after the anchor. A routine is "due" at
//! `now` when that fire falls at or before `now`.
//!
//! Evaluation is O(1) per routine — only the first fire after the anchor is
//! computed — so a routine whose anchor is far in the past never unrolls a long
//! backlog here. The materializer enqueues at most one wake per due routine per
//! tick and advances the anchor to the fire it acted on, so missed periods are
//! collapsed rather than replayed.

use chrono::{DateTime, TimeZone, Utc};
use cron::Schedule;
use std::str::FromStr;

/// The first cron fire strictly after `anchor_unix_seconds`, or `None` if the
/// expression cannot be parsed. A malformed expression is treated as "never due"
/// rather than aborting materialization of every other routine.
///
/// Standard 5-field Unix cron (`min hour dom month dow`) is accepted by
/// prepending a `0` seconds field; the underlying `cron` crate otherwise
/// requires a leading seconds field. 6- and 7-field expressions pass through.
pub fn next_fire_after(expression: &str, anchor_unix_seconds: u64) -> Option<u64> {
    let normalized = normalize_expression(expression);
    let schedule = Schedule::from_str(&normalized).ok()?;
    let anchor = unix(anchor_unix_seconds)?;
    schedule
        .after(&anchor)
        .next()
        .map(|fire| fire.timestamp().max(0) as u64)
}

/// Prepend a `0` seconds field to a standard 5-field Unix cron expression so the
/// `cron` crate accepts it; leave 6/7-field expressions unchanged.
fn normalize_expression(expression: &str) -> String {
    if expression.split_whitespace().count() == 5 {
        format!("0 {expression}")
    } else {
        expression.to_string()
    }
}

/// True when a cron fire occurs in the half-open interval
/// `(anchor_unix_seconds, now_unix_seconds]`.
pub fn is_due(expression: &str, anchor_unix_seconds: u64, now_unix_seconds: u64) -> bool {
    next_fire_after(expression, anchor_unix_seconds)
        .map(|fire| fire <= now_unix_seconds)
        .unwrap_or(false)
}

fn unix(seconds: u64) -> Option<DateTime<Utc>> {
    Utc.timestamp_opt(seconds as i64, 0).single()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Daily at 09:00 UTC. Anchored at midnight, the next fire is 09:00 the same
    /// day; due at/after 09:00, not before.
    #[test]
    fn daily_expression_is_due_after_the_first_fire() {
        let midnight = 0;
        let nine = 9 * 60 * 60;
        assert_eq!(next_fire_after("0 9 * * *", midnight), Some(nine));
        assert!(!is_due("0 9 * * *", midnight, nine - 1));
        assert!(is_due("0 9 * * *", midnight, nine));
        assert!(is_due("0 9 * * *", midnight, nine + 3600));
    }

    /// The fire used as the next anchor lands strictly after the previous fire,
    /// so successive evaluations advance one period at a time without re-firing.
    #[test]
    fn anchor_advances_past_the_fire_it_acted_on() {
        let nine = 9 * 60 * 60;
        let next_day = nine + 24 * 60 * 60;
        assert_eq!(next_fire_after("0 9 * * *", nine), Some(next_day));
        assert!(!is_due("0 9 * * *", nine, next_day - 1));
        assert!(is_due("0 9 * * *", nine, next_day));
    }

    #[test]
    fn every_minute_expression_fires_soon_after_the_anchor() {
        assert_eq!(next_fire_after("* * * * *", 0), Some(60));
        assert!(is_due("* * * * *", 0, 60));
    }

    #[test]
    fn malformed_expression_is_never_due() {
        assert_eq!(next_fire_after("not a cron", 0), None);
        assert!(!is_due("not a cron", 0, u64::MAX));
    }
}
