//! CP-00-02: Architecture boundary tests.
//!
//! These tests enforce the control-plane / execution-plane ownership split
//! defined in ADR 0003 and the parent plan (§3.1–3.2, §12.2). They must:
//!
//! 1. Fail if a future PR adds `altai-control-plane` or
//!    `altai-control-protocol` as a dependency of `altai-agent-service`.
//! 2. Confirm that the control-plane crate remains a workspace peer, rather
//!    than an execution-plane dependency.
//! 3. Self-verify that the detection logic would catch a violation.
//!
//! See: docs/PAPERCLIP_STYLE_CONTROL_PLANE_ENGINEERING_PLAN.md CP-00 exit gate
//! and §12.2 for the full rule set.

use std::collections::HashSet;
use std::fs;

/// Crates that `altai-agent-service` (the execution plane) must never depend
/// on. They are owned by the control plane.
const FORBIDDEN_AGENT_SERVICE_DEPS: &[&str] = &["altai-control-plane", "altai-control-protocol"];

/// Crates that the main `altai` Tauri app binary may depend on (it is the
/// host adapter, not a control-plane owner), but it must not import
/// control-plane *persistence* internals directly. This list is intentionally
/// empty until CP-02 creates the crate; the rule is structural.
const _FORBIDDEN_APP_DEPS: &[&str] = &[];

fn workspace_manifest_path() -> &'static str {
    env!("CARGO_MANIFEST_DIR")
}

/// Read a Cargo.toml file and extract all dependency names from `[dependencies]`
/// and `[dev-dependencies]` sections. Returns a set of crate names (dashes, not
/// underscores — matching how they appear in Cargo.toml).
fn extract_deps(toml_text: &str) -> HashSet<String> {
    let mut deps = HashSet::new();
    let mut in_deps = false;
    let mut in_dev_deps = false;
    for line in toml_text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_deps = trimmed == "[dependencies]";
            in_dev_deps = trimmed == "[dev-dependencies]";
            continue;
        }
        if (in_deps || in_dev_deps) && !trimmed.is_empty() && !trimmed.starts_with('#') {
            if let Some(eq_pos) = trimmed.find('=') {
                let name = trimmed[..eq_pos].trim();
                if !name.is_empty() {
                    deps.insert(name.to_string());
                }
            }
        }
    }
    deps
}

/// Extract workspace member paths from the root Cargo.toml.
fn extract_workspace_members(toml_text: &str) -> Vec<String> {
    let mut members = Vec::new();
    let mut in_workspace = false;
    let mut in_members_array = false;
    for line in toml_text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_workspace = trimmed == "[workspace]";
            in_members_array = false;
            continue;
        }
        if in_workspace && trimmed.starts_with("members") {
            // Find the start of the array; entries may be on this line or
            // following lines until `]` is found.
            if let Some(start) = trimmed.find('[') {
                let after = &trimmed[start + 1..];
                if let Some(end) = after.find(']') {
                    // Single-line array
                    for entry in after[..end].split(',') {
                        let e = entry.trim().trim_matches('"').trim_matches('\'');
                        if !e.is_empty() {
                            members.push(e.to_string());
                        }
                    }
                } else {
                    // Multi-line array: collect entries until closing `]`
                    in_members_array = true;
                    for entry in after.split(',') {
                        let e = entry.trim().trim_matches('"').trim_matches('\'');
                        if !e.is_empty() {
                            members.push(e.to_string());
                        }
                    }
                }
            }
        } else if in_members_array {
            // Continuation of a multi-line members array
            let upto = trimmed.find(']').map(|i| &trimmed[..i]).unwrap_or(trimmed);
            for entry in upto.split(',') {
                let e = entry.trim().trim_matches('"').trim_matches('\'');
                if !e.is_empty() {
                    members.push(e.to_string());
                }
            }
            if trimmed.contains(']') {
                in_members_array = false;
            }
        }
    }
    members
}

#[test]
fn altai_agent_service_does_not_import_control_plane_crates() {
    let manifest_dir = workspace_manifest_path();
    let agent_service_toml =
        std::path::Path::new(manifest_dir).join("crates/altai-agent-service/Cargo.toml");

    let toml_text = fs::read_to_string(&agent_service_toml).unwrap_or_else(|_| {
        panic!(
            "expected altai-agent-service Cargo.toml at {}",
            agent_service_toml.display()
        )
    });

    let deps = extract_deps(&toml_text);

    for forbidden in FORBIDDEN_AGENT_SERVICE_DEPS {
        assert!(
            !deps.contains(*forbidden),
            "Architecture violation (ADR 0003): altai-agent-service depends on `{}`, \
             which is a control-plane crate. The execution plane must not import \
             control-plane persistence or domain modules.",
            forbidden,
        );
    }
}

#[test]
fn workspace_members_include_expected_crates() {
    let root_toml = std::path::Path::new(workspace_manifest_path()).join("Cargo.toml");

    let toml_text = fs::read_to_string(&root_toml).expect("root Cargo.toml must exist");
    let members = extract_workspace_members(&toml_text);

    // The execution-plane, host, shared-contract and control-plane bootstrap
    // crates. The control plane is a workspace peer, never an
    // `altai-agent-service` dependency.
    let expected = [
        "crates/altai-core",
        "crates/altai-agent-service",
        "crates/altai-collaboration",
        "crates/altai-protocol",
        "crates/altai-control-protocol",
        "crates/altai-control-plane",
        "crates/altai-cli",
    ];

    for exp in &expected {
        assert!(
            members.iter().any(|m| m == exp),
            "expected workspace member `{}` not found in root Cargo.toml",
            exp,
        );
    }
}

#[test]
fn altai_control_plane_is_a_workspace_peer_not_execution_dependency() {
    // The M2 skeleton establishes `altai-control-plane` as a workspace peer.
    // The earlier absence assertion is intentionally replaced: future changes
    // must preserve this separation instead of making the execution service
    // depend on control-plane state.
    let root_toml = std::path::Path::new(workspace_manifest_path()).join("Cargo.toml");

    let toml_text = fs::read_to_string(&root_toml).expect("root Cargo.toml must exist");
    let members = extract_workspace_members(&toml_text);

    assert!(
        members
            .iter()
            .any(|m| m == "crates/altai-control-plane" || m == "crates/altai_control_plane"),
        "expected altai-control-plane workspace member",
    );
    let agent_service_toml = std::path::Path::new(workspace_manifest_path())
        .join("crates/altai-agent-service/Cargo.toml");
    let agent_service_deps = extract_deps(
        &fs::read_to_string(&agent_service_toml).expect("agent service Cargo.toml must exist"),
    );
    assert!(
        !agent_service_deps.contains("altai-control-plane"),
        "altai-agent-service must not depend on altai-control-plane",
    );
}

// ---------------------------------------------------------------------------
// Self-verification: prove the detection logic catches violations.
// ---------------------------------------------------------------------------

#[test]
fn self_test_extract_deps_catches_known_dependency() {
    let fake_toml = r#"
[package]
name = "fake"

[dependencies]
serde = "1"
serde_json = "1"

[dev-dependencies]
filetime = "0.2"
"#;
    let deps = extract_deps(fake_toml);
    assert!(deps.contains("serde"));
    assert!(deps.contains("serde_json"));
    assert!(deps.contains("filetime"));
    assert!(!deps.contains("nonexistent"));
}

#[test]
fn self_test_extract_deps_would_detect_forbidden() {
    // Simulate a violation: if altai-agent-service added altai-control-plane,
    // the extractor must find it.
    let violating_toml = r#"
[package]
name = "altai-agent-service"

[dependencies]
altai-core = { path = "../altai-core" }
altai-control-plane = { path = "../altai-control-plane" }
"#;
    let deps = extract_deps(violating_toml);
    assert!(
        deps.contains("altai-control-plane"),
        "self-test: extract_deps must detect altai-control-plane if added",
    );
}

#[test]
fn self_test_extract_workspace_members_parses_array() {
    let fake_toml = r#"
[workspace]
members = [
    "crates/altai-core",
    "crates/altai-agent-service",
]
"#;
    let members = extract_workspace_members(fake_toml);
    assert!(members.iter().any(|m| m == "crates/altai-core"));
    assert!(members.iter().any(|m| m == "crates/altai-agent-service"));
}
