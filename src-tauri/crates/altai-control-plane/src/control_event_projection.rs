//! Projection seed for the control-event log. `fold_checkpoints` is a pure
//! last-write-wins fold over a replay window: it returns each aggregate's
//! high-water mark (last seen sequence and revision). That is the resume
//! point a recovery path or a domain projection (inbox, board rollup)
//! rebuilds from — replay from a checkpoint reconstructs projection state
//! without re-reading the whole log.

use altai_control_protocol::{ControlEvent, Revision};
use std::collections::HashMap;

/// Where one aggregate currently is: the highest applied event sequence and
/// the revision that event left behind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregateCheckpoint {
    pub aggregate: String,
    pub aggregate_id: serde_json::Value,
    pub sequence: u64,
    pub revision: Revision,
}

/// Fold replayed events into per-aggregate checkpoints. Later events win,
/// keyed by `(aggregate, aggregate_id)`; input order (the global replay
/// order) defines "later". Folding a second window over the checkpoints of
/// the first continues the same state — the fold is incremental by design.
pub fn fold_checkpoints(
    checkpoints: HashMap<(String, serde_json::Value), AggregateCheckpoint>,
    events: &[ControlEvent],
) -> HashMap<(String, serde_json::Value), AggregateCheckpoint> {
    let mut folded = checkpoints;
    for event in events {
        let key = (event.aggregate.clone(), event.aggregate_id.clone());
        let is_new_or_later = folded
            .get(&key)
            .is_none_or(|current| current.sequence <= event.sequence);
        if is_new_or_later {
            folded.insert(
                key,
                AggregateCheckpoint {
                    aggregate: event.aggregate.clone(),
                    aggregate_id: event.aggregate_id.clone(),
                    sequence: event.sequence,
                    revision: event.revision,
                },
            );
        }
    }
    folded
}

#[cfg(test)]
mod tests {
    use super::*;
    use altai_control_protocol::{Actor, EventKind};

    fn event(aggregate: &str, aggregate_id: &str, sequence: u64) -> ControlEvent {
        ControlEvent {
            aggregate: aggregate.to_string(),
            aggregate_id: serde_json::json!({ "value": aggregate_id }),
            sequence,
            kind: EventKind::Created,
            actor: Actor::System {
                component: "test".into(),
            },
            timestamp: "2026-08-15T00:00:00Z".into(),
            revision: Revision::new(sequence),
            payload: serde_json::json!({ "sequence": sequence }),
            correlation_id: None,
            causation_id: None,
        }
    }

    fn key(aggregate: &str, aggregate_id: &str) -> (String, serde_json::Value) {
        (aggregate.to_string(), serde_json::json!({ "value": aggregate_id }))
    }

    #[test]
    fn fold_tracks_each_aggregate_independently() {
        let folded = fold_checkpoints(
            HashMap::new(),
            &[
                event("work_item", "wi_1", 1),
                event("attempt", "at_1", 1),
                event("work_item", "wi_1", 2),
                event("work_item", "wi_2", 1),
            ],
        );
        assert_eq!(folded.len(), 3);
        let wi_1 = &folded[&key("work_item", "wi_1")];
        assert_eq!(wi_1.sequence, 2);
        assert_eq!(wi_1.revision, Revision::new(2));
        assert_eq!(folded[&key("attempt", "at_1")].sequence, 1);
    }

    #[test]
    fn fold_is_incremental_across_windows() {
        let first = fold_checkpoints(
            HashMap::new(),
            &[event("work_item", "wi_1", 1), event("work_item", "wi_1", 2)],
        );
        // A later window replays a stale event (already applied) plus new
        // state; the stale one must not move the checkpoint backwards.
        let second = fold_checkpoints(
            first,
            &[event("work_item", "wi_1", 1), event("work_item", "wi_1", 3)],
        );
        let wi_1 = &second[&key("work_item", "wi_1")];
        assert_eq!(wi_1.sequence, 3);
        assert_eq!(wi_1.revision, Revision::new(3));
        assert_eq!(second.len(), 1);
    }
}
