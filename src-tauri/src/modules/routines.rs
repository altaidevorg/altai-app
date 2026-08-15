//! Scheduled-work projection (package 066, PR 1). Routine aggregates are
//! durable in `work.db` — the control plane owns their writes, the cron
//! bridge materializes them into wakes — but the desktop had no way to
//! read them. This module is that read side: one projection joins each
//! routine to its current intent (trigger, target Work) and its firing
//! state (last materialized fire, the next fire the bridge will act on,
//! computed with the materializer's own anchor semantics). Read-only:
//! lifecycle moves stay control-plane commands.

use altai_control_plane::{
    cron_due, LegacyWorkBridge, RoutineRepository, SqliteRoutineRepository,
};
use altai_control_protocol::{Routine, RoutineRevision, RoutineStatus, RoutineTrigger};
use serde::Serialize;
use tauri::State;

use crate::modules::work::{authorized_workspace, open_store, resolve_work_db};
use crate::modules::workspace::WorkspaceRegistry;

/// A routine joined with its current intent and firing state — the row a
/// scheduled-work surface renders.
#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RoutineDto {
    pub id: String,
    pub status: &'static str,
    pub revision: u64,
    pub trigger_kind: &'static str,
    pub cron_expression: Option<String>,
    pub event_source: Option<String>,
    pub target_work_id: Option<String>,
    pub target_work_title: Option<String>,
    /// The most recent fire the cron bridge materialized, if it ever did.
    pub last_fired_at_ms: Option<u64>,
    /// The next fire the bridge will act on: the first cron match after
    /// the anchor (`last_fired`, else the revision's creation). It may be
    /// in the past — an overdue routine is one the bridge has not run to
    /// catch up on, which the surface should say rather than hide.
    pub next_fire_at_ms: Option<u64>,
    pub updated_at_ms: u64,
}

fn status_label(status: RoutineStatus) -> &'static str {
    match status {
        RoutineStatus::Active => "active",
        RoutineStatus::Paused => "paused",
        RoutineStatus::Retired => "retired",
    }
}

/// Project one routine. Pure: firing state is computed from durable
/// facts with exactly the anchor semantics the materializer uses, so
/// what the surface shows as "next" is what the bridge will do next.
fn project_routine(
    routine: &Routine,
    revision: Option<&RoutineRevision>,
    last_fired_unix_seconds: Option<u64>,
    target_work_title: Option<&str>,
) -> RoutineDto {
    let mut dto = RoutineDto {
        id: routine.id.value.clone(),
        status: status_label(routine.status),
        revision: routine.revision.0,
        trigger_kind: "recurring",
        cron_expression: None,
        event_source: None,
        target_work_id: None,
        target_work_title: None,
        last_fired_at_ms: last_fired_unix_seconds.map(|s| s * 1000),
        next_fire_at_ms: None,
        updated_at_ms: routine.updated_at_unix_seconds * 1000,
    };
    let Some(revision) = revision else {
        // Created but never given intent: nothing is scheduled yet.
        return dto;
    };
    dto.target_work_id = Some(revision.target_work_item_id.value.clone());
    dto.target_work_title = target_work_title.map(Into::into);
    match &revision.trigger {
        RoutineTrigger::Recurring { cron_expression } => {
            dto.cron_expression = Some(cron_expression.clone());
            let anchor = last_fired_unix_seconds
                .unwrap_or(revision.created_at_unix_seconds);
            dto.next_fire_at_ms =
                cron_due::next_fire_after(cron_expression, anchor).map(|s| s * 1000);
        }
        RoutineTrigger::Event { source } => {
            dto.trigger_kind = "event";
            dto.event_source = Some(source.clone());
        }
    }
    dto
}

#[tauri::command]
pub fn routines_list(
    registry: State<'_, WorkspaceRegistry>,
    workspace_path: String,
) -> Result<Vec<RoutineDto>, String> {
    let workspace = authorized_workspace(&workspace_path, &registry)?;
    let (_project_id, store) = open_store(&registry, &workspace)?;
    let database = resolve_work_db(&workspace)?;
    let repository =
        SqliteRoutineRepository::open(&database).map_err(|error| error.to_string())?;
    // Routine targets are canonical control-plane WorkItem ids; the desktop
    // WorkStore holds legacy ids. The one-way bridge owns the mapping table,
    // so the title join resolves through it — a target with no mapping yet
    // keeps its raw id and simply is not drillable.
    let bridge = LegacyWorkBridge::open(&database).map_err(|error| error.to_string())?;
    let routines = repository.list_all().map_err(|error| error.to_string())?;
    let mut rows = Vec::with_capacity(routines.len());
    for routine in &routines {
        let revision = match &routine.current_revision_id {
            Some(revision_id) => repository
                .get_revision(revision_id)
                .map_err(|error| error.to_string())?,
            None => None,
        };
        let last_fired = repository
            .last_fired(&routine.id)
            .map_err(|error| error.to_string())?;
        let target_work_title = match &revision {
            Some(revision) => bridge
                .legacy_id_for(&revision.target_work_item_id)
                .map_err(|error| error.to_string())?
                .and_then(|legacy_id| {
                    store.get_work(&legacy_id).ok().flatten().map(|work| work.title)
                }),
            None => None,
        };
        rows.push(project_routine(
            routine,
            revision.as_ref(),
            last_fired,
            target_work_title.as_deref(),
        ));
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::project_routine;
    use altai_control_protocol::{
        OrganizationId, Revision, Routine, RoutineId, RoutineRevision, RoutineRevisionId,
        RoutineStatus, RoutineTrigger, WorkItemId,
    };

    fn routine(status: RoutineStatus, current_revision_id: Option<RoutineRevisionId>) -> Routine {
        Routine {
            id: RoutineId::new("rt_1"),
            organization_id: OrganizationId::new("org"),
            current_revision_id,
            status,
            revision: Revision(3),
            created_at_unix_seconds: 100,
            updated_at_unix_seconds: 400,
        }
    }

    fn recurring_revision(created_at_unix_seconds: u64) -> RoutineRevision {
        RoutineRevision {
            id: RoutineRevisionId::new("rtr_1"),
            routine_id: RoutineId::new("rt_1"),
            revision: Revision(1),
            trigger: RoutineTrigger::Recurring {
                // Daily at 09:00 UTC.
                cron_expression: "0 9 * * *".into(),
            },
            target_work_item_id: WorkItemId::new("work_1"),
            created_at_unix_seconds,
        }
    }

    #[test]
    fn recurring_routine_anchors_next_fire_on_its_last_materialized_fire() {
        let revision = recurring_revision(100);
        let dto = project_routine(
            &routine(RoutineStatus::Active, Some(revision.id.clone())),
            Some(&revision),
            // Last fire: day 2 at 09:00 (205_200 = 2×86_400 + 32_400).
            Some(205_200),
            Some("Nightly digest"),
        );
        assert_eq!(dto.trigger_kind, "recurring");
        assert_eq!(dto.cron_expression.as_deref(), Some("0 9 * * *"));
        assert_eq!(dto.last_fired_at_ms, Some(205_200_000));
        // Strictly after the anchor: day 3 at 09:00 = 291_600.
        assert_eq!(dto.next_fire_at_ms, Some(291_600_000));
        // Typed ids normalize with their prefix; the row keeps the raw id.
        assert_eq!(dto.target_work_id.as_deref(), Some("wi_work_1"));
        assert_eq!(dto.target_work_title.as_deref(), Some("Nightly digest"));
        assert_eq!(dto.status, "active");
        assert_eq!(dto.updated_at_ms, 400_000);
    }

    #[test]
    fn never_fired_routine_anchors_next_fire_on_its_revision_creation() {
        let revision = recurring_revision(0);
        let dto = project_routine(
            &routine(RoutineStatus::Paused, Some(revision.id.clone())),
            Some(&revision),
            None,
            None,
        );
        // Anchor 0 → first fire at 09:00 the same day.
        assert_eq!(dto.next_fire_at_ms, Some(32_400_000));
        assert_eq!(dto.last_fired_at_ms, None);
        // The target Work may not exist (yet) — the row still renders.
        assert_eq!(dto.target_work_title, None);
        assert_eq!(dto.status, "paused");
    }

    #[test]
    fn event_triggered_routine_has_no_cron_facts() {
        let mut revision = recurring_revision(100);
        revision.trigger = RoutineTrigger::Event {
            source: "github.pull_request.opened".into(),
        };
        let dto = project_routine(
            &routine(RoutineStatus::Active, Some(revision.id.clone())),
            Some(&revision),
            None,
            None,
        );
        assert_eq!(dto.trigger_kind, "event");
        assert_eq!(dto.event_source.as_deref(), Some("github.pull_request.opened"));
        assert_eq!(dto.cron_expression, None);
        assert_eq!(dto.next_fire_at_ms, None);
    }

    #[test]
    fn malformed_cron_degrades_to_no_next_fire() {
        let mut revision = recurring_revision(100);
        revision.trigger = RoutineTrigger::Recurring {
            cron_expression: "not a cron".into(),
        };
        let dto = project_routine(
            &routine(RoutineStatus::Active, Some(revision.id.clone())),
            Some(&revision),
            None,
            None,
        );
        assert_eq!(dto.next_fire_at_ms, None);
        assert_eq!(dto.cron_expression.as_deref(), Some("not a cron"));
    }

    #[test]
    fn routine_without_intent_schedules_nothing() {
        let dto = project_routine(
            &routine(RoutineStatus::Active, None),
            None,
            Some(50),
            None,
        );
        assert_eq!(dto.trigger_kind, "recurring");
        assert_eq!(dto.cron_expression, None);
        assert_eq!(dto.target_work_id, None);
        // A fire recorded before any intent is still a durable fact.
        assert_eq!(dto.last_fired_at_ms, Some(50_000));
    }
}
