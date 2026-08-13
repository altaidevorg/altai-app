//! CAL-02 calibration: prove Rust round-trips the shared control-plane
//! golden fixture byte-identically before the real CP-01 contracts exist.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::PathBuf;

const WORK_ITEM_ID_TYPE: &str = "work_item_id";
const WORK_ITEM_ID_PREFIX: &str = "wi_";
const CANONICAL_JSON: &str =
    r#"{"type":"work_item_id","value":"wi_01923abc-def0-7abc-8def-0123456789ab"}"#;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../shared/control-protocol/v1/fixtures/work-item-id.json")
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct WorkItemId {
    #[serde(rename = "type")]
    kind: String,
    value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum WorkItemIdError {
    InvalidJson(String),
    WrongType(String),
    EmptyValue,
    MissingPrefix(String),
}

impl fmt::Display for WorkItemIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidJson(error) => write!(f, "invalid JSON: {error}"),
            Self::WrongType(kind) => write!(f, "expected type {WORK_ITEM_ID_TYPE:?}, got {kind:?}"),
            Self::EmptyValue => write!(f, "work item id value is empty"),
            Self::MissingPrefix(value) => {
                write!(f, "work item id {value:?} is missing the {WORK_ITEM_ID_PREFIX:?} prefix")
            }
        }
    }
}

impl std::error::Error for WorkItemIdError {}

fn parse_work_item_id(json: &str) -> Result<WorkItemId, WorkItemIdError> {
    let parsed: WorkItemId =
        serde_json::from_str(json).map_err(|error| WorkItemIdError::InvalidJson(error.to_string()))?;
    if parsed.kind != WORK_ITEM_ID_TYPE {
        return Err(WorkItemIdError::WrongType(parsed.kind));
    }
    if parsed.value.is_empty() {
        return Err(WorkItemIdError::EmptyValue);
    }
    if !parsed.value.starts_with(WORK_ITEM_ID_PREFIX) {
        return Err(WorkItemIdError::MissingPrefix(parsed.value));
    }
    Ok(parsed)
}

#[test]
fn round_trips_shared_work_item_id_fixture_byte_identically() {
    let fixture = fs::read_to_string(fixture_path()).expect("fixture read");
    let parsed = parse_work_item_id(&fixture).expect("fixture parses");
    let serialized = serde_json::to_string(&parsed).expect("fixture serializes");
    assert_eq!(serialized, CANONICAL_JSON);
    let reparsed = parse_work_item_id(&serialized).expect("canonical JSON parses");
    assert_eq!(reparsed, parsed);
    assert_eq!(
        serde_json::to_string(&reparsed).expect("re-serializes"),
        serialized
    );
}

#[test]
fn rejects_empty_work_item_id_value_with_typed_error() {
    let result = parse_work_item_id(r#"{"type":"work_item_id","value":""}"#);
    assert_eq!(result, Err(WorkItemIdError::EmptyValue));
}

#[test]
fn rejects_missing_prefix_with_typed_error() {
    let result = parse_work_item_id(r#"{"type":"work_item_id","value":"no-prefix"}"#);
    assert_eq!(
        result,
        Err(WorkItemIdError::MissingPrefix("no-prefix".to_string()))
    );
}

#[test]
fn rejects_wrong_type_with_typed_error() {
    let result = parse_work_item_id(r#"{"type":"goal_id","value":"wi_01923abc-def0-7abc-8def-0123456789ab"}"#);
    assert_eq!(result, Err(WorkItemIdError::WrongType("goal_id".to_string())));
}
