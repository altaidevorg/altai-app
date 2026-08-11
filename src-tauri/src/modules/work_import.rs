//! Read-only Tauri bridge for the Work OS legacy import preview.

use std::path::Path;

use altai_core::{
    preview_legacy_import, resolve_workspace_from, LegacyImportPreviewPaths,
    LegacyImportPreviewReport, LegacyImportSource, LegacySqliteSource, LegacyWorkspaceRoot,
};
use tauri::{AppHandle, Manager, State};

use crate::modules::workspace::{OpenedWorkspaceGrant, WorkspaceRegistry};

fn contained_existing_source(
    base: &Path,
    source: std::path::PathBuf,
) -> Result<LegacyImportSource, String> {
    match std::fs::symlink_metadata(&source) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err("legacy_source_type_rejected".into())
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(LegacyImportSource::missing(source))
        }
        Err(_error) => return Err("legacy_source_inspection_failed".into()),
    }
    let captured = LegacyImportSource::capture(source.clone())?;
    let canonical_base = base
        .canonicalize()
        .map_err(|_error| "legacy_source_root_resolution_failed".to_string())?;
    let canonical_source = source
        .canonicalize()
        .map_err(|_error| "legacy_source_resolution_failed".to_string())?;
    if !canonical_source.starts_with(&canonical_base) {
        return Err("legacy_source_outside_authorized_root".into());
    }
    let canonical_metadata = canonical_source
        .symlink_metadata()
        .map_err(|_error| "legacy_source_identity_unavailable".to_string())?;
    let expected = captured
        .expected_identity
        .as_ref()
        .ok_or_else(|| "legacy_source_identity_unavailable".to_string())?;
    if canonical_metadata.file_type().is_symlink() || !expected.matches(&canonical_metadata) {
        return Err("legacy_source_identity_changed".into());
    }
    Ok(LegacyImportSource {
        path: canonical_source,
        expected_identity: captured.expected_identity,
    })
}

fn contained_sqlite_source(
    base: &Path,
    path: std::path::PathBuf,
) -> Result<LegacySqliteSource, String> {
    let main = contained_existing_source(base, path.clone())?;
    let wal = contained_existing_source(base, sqlite_sidecar_path(&path, "-wal"))?;
    let shm = contained_existing_source(base, sqlite_sidecar_path(&path, "-shm"))?;
    let rollback = contained_existing_source(base, sqlite_sidecar_path(&path, "-journal"))?;
    let wal_present = wal.expected_identity.is_some();
    let shm_present = shm.expected_identity.is_some();
    let rollback_present = rollback.expected_identity.is_some();
    if main.expected_identity.is_none() {
        if wal_present || shm_present || rollback_present {
            return Err("sqlite_orphan_sidecar_rejected".into());
        }
        return Ok(LegacySqliteSource::missing(path));
    }
    if rollback_present {
        return Err("sqlite_rollback_journal_rejected".into());
    }
    if wal_present != shm_present {
        return Err("sqlite_wal_pair_incomplete".into());
    }
    Ok(LegacySqliteSource {
        main,
        wal: wal_present.then_some(wal),
        shm: shm_present.then_some(shm),
    })
}

fn sqlite_sidecar_path(path: &Path, suffix: &str) -> std::path::PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(suffix);
    value.into()
}

fn authorized_workspace(
    workspace_path: &str,
    registry: &WorkspaceRegistry,
) -> Result<OpenedWorkspaceGrant, String> {
    let raw = workspace_path.trim();
    if raw.is_empty() {
        return Err("workspacePath is required".into());
    }
    registry
        .capture_opened_exact(raw)
        .map_err(|_error| "workspace_exact_grant_required".to_string())
}

fn revalidate_workspace_grant(
    registry: &WorkspaceRegistry,
    grant: &OpenedWorkspaceGrant,
) -> Result<(), String> {
    registry
        .is_opened_grant_current(grant)
        .then_some(())
        .ok_or_else(|| "workspace_identity_changed_during_preview_setup".to_string())
}

/// Backend-only dry-run for useful legacy records. This command does not
/// create/migrate `work.db`, modify a source, or expose a user-facing apply
/// flow; preview UI/CLI reporting is intentionally a follow-up slice.
#[tauri::command]
pub async fn work_legacy_import_preview(
    app: AppHandle,
    registry: State<'_, WorkspaceRegistry>,
    workspace_path: String,
) -> Result<LegacyImportPreviewReport, String> {
    let workspace_grant = authorized_workspace(&workspace_path, &registry)?;
    let workspace = workspace_grant.path().to_path_buf();
    let workspace_paths = resolve_workspace_from(Some(&workspace), Path::new(&workspace))
        .map_err(|_error| "workspace_resolution_failed".to_string())?;
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|_error| "app_data_dir_unavailable".to_string())?;
    let assignments_json =
        contained_existing_source(&app_data, app_data.join("altai-assignments.json"))?;
    let todos_json = contained_existing_source(&app_data, app_data.join("altai-ai-todos.json"))?;
    let task_run_journal_db =
        contained_sqlite_source(&workspace, workspace_paths.agent_event_journal_db())?;
    let work_db = contained_sqlite_source(&workspace, workspace_paths.work_db())?;
    let selected_workspace = LegacyWorkspaceRoot::capture(workspace)
        .map_err(|_error| "workspace_identity_changed_during_preview_setup".to_string())?;
    revalidate_workspace_grant(&registry, &workspace_grant)?;
    let paths = LegacyImportPreviewPaths {
        selected_workspace,
        assignments_json,
        todos_json,
        task_run_journal_db,
        work_db,
    };
    let report = tauri::async_runtime::spawn_blocking(move || preview_legacy_import(&paths))
        .await
        .map_err(|_error| "legacy_preview_worker_failed".to_string())?;
    revalidate_workspace_grant(&registry, &workspace_grant)
        .map_err(|_error| "workspace_identity_changed_during_preview".to_string())?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::{
        authorized_workspace, contained_existing_source, contained_sqlite_source,
        revalidate_workspace_grant, sqlite_sidecar_path,
    };
    use crate::modules::workspace::WorkspaceRegistry;
    use altai_core::{
        preview_legacy_import, LegacyImportPreviewPaths, LegacyImportSource, LegacySqliteSource,
        LegacyWorkspaceRoot,
    };

    #[test]
    fn contained_source_allows_missing_child_without_creating_it() {
        let root = tempfile::tempdir().expect("root");
        let source = root.path().join("missing.json");
        let captured =
            contained_existing_source(root.path(), source.clone()).expect("missing is valid");
        assert_eq!(captured.path, source);
        assert!(captured.expected_identity.is_none());
        assert!(!source.exists());
    }

    #[test]
    fn contained_source_rejects_existing_file_outside_authorized_root() {
        let root = tempfile::tempdir().expect("root");
        let outside = tempfile::tempdir().expect("outside");
        let source = outside.path().join("legacy.json");
        std::fs::write(&source, b"{}").expect("source");
        let error = contained_existing_source(root.path(), source.clone()).expect_err("outside");
        assert!(!error.contains(&source.to_string_lossy().into_owned()));
    }

    #[test]
    fn sqlite_source_rejects_an_incomplete_wal_pair() {
        let root = tempfile::tempdir().expect("root");
        let source = root.path().join("work.db");
        std::fs::write(&source, b"main").expect("main");
        std::fs::write(sqlite_sidecar_path(&source, "-wal"), b"wal").expect("wal");

        assert_eq!(
            contained_sqlite_source(root.path(), source).expect_err("incomplete pair"),
            "sqlite_wal_pair_incomplete"
        );
    }

    #[test]
    fn sqlite_source_rejects_an_existing_rollback_journal() {
        let root = tempfile::tempdir().expect("root");
        let source = root.path().join("work.db");
        std::fs::write(&source, b"main").expect("main");
        std::fs::write(sqlite_sidecar_path(&source, "-journal"), b"rollback")
            .expect("rollback journal");

        assert_eq!(
            contained_sqlite_source(root.path(), source).expect_err("rollback journal"),
            "sqlite_rollback_journal_rejected"
        );
    }

    #[cfg(unix)]
    #[test]
    fn sqlite_source_rejects_a_symlinked_sidecar() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().expect("root");
        let source = root.path().join("work.db");
        let target = root.path().join("wal-target");
        std::fs::write(&source, b"main").expect("main");
        std::fs::write(&target, b"wal").expect("wal target");
        symlink(&target, sqlite_sidecar_path(&source, "-wal")).expect("wal symlink");
        std::fs::write(sqlite_sidecar_path(&source, "-shm"), b"shm").expect("shm");

        assert_eq!(
            contained_sqlite_source(root.path(), source).expect_err("symlink sidecar"),
            "legacy_source_type_rejected"
        );
    }

    #[test]
    fn captured_identity_rejects_parent_substitution_without_parsing_outside_file() {
        let parent = tempfile::tempdir().expect("parent");
        let base = parent.path().join("app-data");
        let moved = parent.path().join("authorized-original");
        std::fs::create_dir(&base).expect("base");
        let source = base.join("altai-assignments.json");
        std::fs::write(&source, br#"{"assignments":[]}"#).expect("authorized source");
        let captured = contained_existing_source(&base, source).expect("capture identity");

        std::fs::rename(&base, &moved).expect("move authorized parent");
        std::fs::create_dir(&base).expect("replacement parent");
        std::fs::write(
            base.join("altai-assignments.json"),
            br#"{"assignments":[{"id":"outside","source":{"kind":"task","prompt":"p"},"sessionId":"s","title":"OUTSIDE_SENTINEL","status":"running","runConfig":{"workspacePath":"/"},"createdAt":1,"updatedAt":1}]}"#,
        )
        .expect("outside replacement");

        let report = preview_legacy_import(&LegacyImportPreviewPaths {
            selected_workspace: LegacyWorkspaceRoot::capture(base.clone()).expect("root token"),
            assignments_json: captured,
            todos_json: LegacyImportSource::missing(base.join("todos.json")),
            task_run_journal_db: LegacySqliteSource::missing(base.join("journal.db")),
            work_db: LegacySqliteSource::missing(base.join("work.db")),
        });
        assert_eq!(report.candidate_count, 0);
        assert_eq!(report.error_count, 1);
        assert!(report
            .items
            .iter()
            .all(|item| item.title.as_deref() != Some("OUTSIDE_SENTINEL")));
        assert_eq!(report.items[0].reason, "legacy_source_identity_changed");
    }

    #[test]
    fn preview_requires_current_exact_opened_grant_across_probe_switch_and_revoke() {
        let root = tempfile::tempdir().expect("root");
        let probe = tempfile::tempdir().expect("probe");
        let replacement = tempfile::tempdir().expect("replacement");
        let registry = WorkspaceRegistry::default();
        let canonical = registry.authorize(root.path()).expect("broad grant");
        let canonical_probe = registry.authorize(probe.path()).expect("broad probe");
        let path = canonical.to_string_lossy().into_owned();
        assert!(authorized_workspace(&path, &registry).is_err());
        assert!(authorized_workspace(&canonical_probe.to_string_lossy(), &registry).is_err());

        registry
            .authorize_opened(&canonical)
            .expect("exact opened grant");
        let grant = authorized_workspace(&path, &registry).expect("preview authorization");
        assert_eq!(grant.path(), canonical);

        let canonical_replacement = registry
            .authorize_opened(replacement.path())
            .expect("switch exact grant");
        assert!(authorized_workspace(&path, &registry).is_err());
        assert_eq!(
            revalidate_workspace_grant(&registry, &grant).expect_err("stale switched grant"),
            "workspace_identity_changed_during_preview_setup"
        );
        assert!(authorized_workspace(&canonical_replacement.to_string_lossy(), &registry).is_ok());
        let replacement_grant =
            authorized_workspace(&canonical_replacement.to_string_lossy(), &registry)
                .expect("replacement preview authorization");

        registry.revoke_opened();
        assert!(authorized_workspace(&canonical_replacement.to_string_lossy(), &registry).is_err());
        assert_eq!(
            revalidate_workspace_grant(&registry, &replacement_grant)
                .expect_err("revoked captured grant"),
            "workspace_identity_changed_during_preview_setup"
        );
    }

    #[test]
    fn final_grant_check_rejects_root_swap_after_source_capture() {
        let parent = tempfile::tempdir().expect("parent");
        let workspace = parent.path().join("workspace");
        let moved = parent.path().join("workspace-original");
        std::fs::create_dir(&workspace).expect("workspace");
        let registry = WorkspaceRegistry::default();
        registry
            .authorize_opened(&workspace)
            .expect("opened workspace");
        let grant =
            authorized_workspace(&workspace.to_string_lossy(), &registry).expect("captured grant");

        std::fs::rename(&workspace, &moved).expect("move workspace");
        std::fs::create_dir(&workspace).expect("replacement workspace");
        let source = workspace.join("source.json");
        std::fs::write(&source, b"OUTSIDE_SENTINEL").expect("replacement source");
        let _captured =
            contained_existing_source(&workspace, source).expect("capture replacement source");

        let error = revalidate_workspace_grant(&registry, &grant).expect_err("root swap");
        assert_eq!(error, "workspace_identity_changed_during_preview_setup");
        assert!(!error.contains("OUTSIDE_SENTINEL"));
    }

    #[test]
    fn core_root_token_rejects_swap_after_final_setup_check_before_use() {
        let parent = tempfile::tempdir().expect("parent");
        let workspace = parent.path().join("workspace");
        let moved = parent.path().join("workspace-original");
        std::fs::create_dir(&workspace).expect("workspace");
        let assignments = workspace.join("assignments.json");
        std::fs::write(&assignments, br#"{"assignments":[]}"#).expect("trusted source");
        let registry = WorkspaceRegistry::default();
        registry
            .authorize_opened(&workspace)
            .expect("opened workspace");
        let grant =
            authorized_workspace(&workspace.to_string_lossy(), &registry).expect("captured grant");
        let captured_assignments =
            contained_existing_source(&workspace, assignments).expect("captured source");
        let root_token =
            LegacyWorkspaceRoot::capture(workspace.clone()).expect("captured root token");
        revalidate_workspace_grant(&registry, &grant).expect("final setup check");

        std::fs::rename(&workspace, &moved).expect("move trusted root");
        std::fs::create_dir(&workspace).expect("replacement root");
        std::fs::write(
            workspace.join("assignments.json"),
            br#"{"assignments":[{"id":"outside","source":{"kind":"task","prompt":"p"},"sessionId":"s","title":"OUTSIDE_SENTINEL","status":"running","runConfig":{"workspacePath":"/"},"createdAt":1,"updatedAt":1}]}"#,
        )
        .expect("replacement source");

        let report = preview_legacy_import(&LegacyImportPreviewPaths {
            selected_workspace: root_token,
            assignments_json: captured_assignments,
            todos_json: LegacyImportSource::missing(workspace.join("todos.json")),
            task_run_journal_db: LegacySqliteSource::missing(workspace.join("journal.db")),
            work_db: LegacySqliteSource::missing(workspace.join("work.db")),
        });
        assert_eq!(report.candidate_count, 0);
        assert_eq!(report.error_count, 1);
        assert_eq!(
            report.items[0].reason,
            "workspace_identity_changed_during_preview"
        );
        assert!(report
            .items
            .iter()
            .all(|item| item.title.as_deref() != Some("OUTSIDE_SENTINEL")));
        assert!(revalidate_workspace_grant(&registry, &grant).is_err());
    }
}
