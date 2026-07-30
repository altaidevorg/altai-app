use altai_protocol::validate_message;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../shared/agent-protocol/v1/fixtures")
}

#[test]
fn validates_shared_golden_fixtures() {
    for entry in fs::read_dir(fixtures_dir()).expect("fixture directory") {
        let path = entry.expect("fixture entry").path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let fixture: Value =
            serde_json::from_slice(&fs::read(&path).expect("fixture read")).expect("fixture JSON");
        let valid = fixture["valid"].as_bool().unwrap_or(true);
        let result = validate_message(fixture["message"].clone());
        if valid {
            result.unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        } else {
            assert!(result.is_err(), "{} should be rejected", path.display());
        }
    }
}
