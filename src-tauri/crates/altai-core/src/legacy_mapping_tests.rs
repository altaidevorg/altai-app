//! CAL-03 tests: positive mapping, required negative cases, and purity.

use crate::legacy_mapping::{
    map_legacy_assignment, ExecutionPhase, LegacyAssignment, LegacyMappingError, WorkStatus,
};

fn legacy_assignment(status: &str) -> LegacyAssignment {
    LegacyAssignment {
        id: Some("legacy-task-42".to_string()),
        title: Some("Fix the thing".to_string()),
        status: Some(status.to_string()),
        created_at: Some("2026-08-03T12:00:00Z".to_string()),
    }
}

#[test]
fn maps_all_known_statuses_per_amended_table() {
    let cases = [
        ("queued", WorkStatus::Todo, ExecutionPhase::Queued),
        ("running", WorkStatus::InProgress, ExecutionPhase::Running),
        ("succeeded", WorkStatus::Done, ExecutionPhase::Terminal),
        ("failed", WorkStatus::InProgress, ExecutionPhase::Failed),
        ("cancelled", WorkStatus::Cancelled, ExecutionPhase::Terminal),
    ];
    for (status, expected_work_status, expected_execution_phase) in cases {
        let mapped = map_legacy_assignment(&legacy_assignment(status))
            .unwrap_or_else(|error| panic!("status {status:?} should map: {error}"));
        assert_eq!(mapped.work_status, expected_work_status, "status {status:?}");
        assert_eq!(
            mapped.execution_phase, expected_execution_phase,
            "status {status:?}"
        );
    }
}

#[test]
fn maps_sample_legacy_assignment_to_canonical_shape() {
    let mapped = map_legacy_assignment(&legacy_assignment("queued")).expect("maps");
    assert_eq!(mapped.title, "Fix the thing");
    assert_eq!(mapped.work_status, WorkStatus::Todo);
    assert_eq!(mapped.execution_phase, ExecutionPhase::Queued);
    assert_eq!(mapped.created_at, "2026-08-03T12:00:00Z");
    // The pure mapping never invents durable IDs.
    assert_eq!(mapped.work_item_id, None);
}

#[test]
fn preserves_legacy_id_in_legacy_compat_id() {
    let mapped = map_legacy_assignment(&legacy_assignment("running")).expect("maps");
    assert_eq!(mapped.legacy_compat_id, "legacy-task-42");
}

#[test]
fn mapping_is_pure_same_input_twice_same_output() {
    let input = legacy_assignment("failed");
    let first = map_legacy_assignment(&input).expect("maps");
    let second = map_legacy_assignment(&input).expect("maps");
    assert_eq!(first, second);
}

#[test]
fn rejects_unknown_status_with_typed_error() {
    let result = map_legacy_assignment(&legacy_assignment("foobar"));
    assert_eq!(
        result,
        Err(LegacyMappingError::UnknownLegacyStatus(
            "foobar".to_string()
        ))
    );
}

#[test]
fn rejects_missing_title_with_typed_error() {
    let mut input = legacy_assignment("queued");
    input.title = None;
    let result = map_legacy_assignment(&input);
    assert_eq!(
        result,
        Err(LegacyMappingError::MissingRequiredField("title"))
    );
}

#[test]
fn rejects_missing_id_with_typed_error() {
    let mut input = legacy_assignment("queued");
    input.id = None;
    let result = map_legacy_assignment(&input);
    assert_eq!(result, Err(LegacyMappingError::MissingRequiredField("id")));
}

#[test]
fn rejects_empty_id_with_typed_error() {
    let mut input = legacy_assignment("queued");
    input.id = Some(String::new());
    let result = map_legacy_assignment(&input);
    assert_eq!(result, Err(LegacyMappingError::InvalidLegacyId));
}

#[test]
fn rejects_null_status_with_typed_error() {
    let mut input = legacy_assignment("queued");
    input.status = None;
    let result = map_legacy_assignment(&input);
    assert_eq!(
        result,
        Err(LegacyMappingError::MissingRequiredField("status"))
    );
}
