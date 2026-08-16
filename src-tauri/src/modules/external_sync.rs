//! Desktop host layer for external-object conflict resolution (package
//! 070, PR 4). The engine's `resolve_external_conflict` owns the rules and
//! the audit event; this module only supplies the host facts they cannot
//! know — the workspace's `work.db`, the local organization, and the
//! clock — behind one command.

use altai_control_plane::{
    resolve_external_conflict, ConflictResolution, SqliteActivityEventRepository,
    SqliteExternalObjectRepository, SqliteScopeRepository,
};
use altai_control_protocol::{ExternalAuthority, ExternalObjectId, ExternalObject};
use serde::Serialize;

use crate::modules::work;

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExternalObjectDto {
    pub external_object_id: String,
    pub integration: String,
    pub external_id: String,
    pub object_kind: String,
    pub title: String,
    pub url: Option<String>,
    pub authority: ExternalAuthority,
    pub refused_content_hash: Option<String>,
    pub declined_content_hash: Option<String>,
}

impl From<ExternalObject> for ExternalObjectDto {
    fn from(object: ExternalObject) -> Self {
        Self {
            external_object_id: object.id.value,
            integration: object.integration,
            external_id: object.external_id,
            object_kind: object.object_kind,
            title: object.title,
            url: object.url,
            authority: object.authority,
            refused_content_hash: object.refused_content_hash,
            declined_content_hash: object.declined_content_hash,
        }
    }
}

/// The command's resolution vocabulary, kept to two spellings so callers
/// cannot invent a third meaning.
fn parse_resolution(raw: &str) -> Result<ConflictResolution, String> {
    match raw {
        "take_external" => Ok(ConflictResolution::TakeExternal),
        "keep_local" => Ok(ConflictResolution::KeepLocal),
        other => Err(format!(
            "unknown resolution '{other}': expected 'take_external' or 'keep_local'"
        )),
    }
}

/// Apply an explicit decision to a refused external-object overwrite: the
/// provider's version (`take_external`) or the local one (`keep_local`).
/// The decision is recorded in the control-plane activity stream.
#[tauri::command]
pub async fn external_object_resolve_conflict(
    workspace_path: String,
    external_object_id: String,
    resolution: String,
) -> Result<ExternalObjectDto, String> {
    let resolution = parse_resolution(&resolution)?;
    let id = ExternalObjectId::new(external_object_id);
    let database = work::resolve_work_db(&workspace_path)?;

    tauri::async_runtime::spawn_blocking(move || {
        let scope = SqliteScopeRepository::open(&database)?;
        let organization = scope
            .ensure_default_local_organization()
            .map_err(|error| error.to_string())?;
        let repository = SqliteExternalObjectRepository::open(&database)?;
        let activity = SqliteActivityEventRepository::open(&database)?;
        let timestamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        let resolved = resolve_external_conflict(
            &organization.id,
            &repository,
            &activity,
            &id,
            resolution,
            &timestamp,
        )
        .map_err(|error| error.to_string())?;
        Ok(ExternalObjectDto::from(resolved))
    })
    .await
    .map_err(|error| format!("conflict resolution join failed: {error}"))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolutions_parse_from_their_two_spellings() {
        assert_eq!(parse_resolution("take_external").unwrap(), ConflictResolution::TakeExternal);
        assert_eq!(parse_resolution("keep_local").unwrap(), ConflictResolution::KeepLocal);
    }

    #[test]
    fn any_other_spelling_is_rejected_with_the_expected_pair() {
        let error = parse_resolution("provider_wins").unwrap_err();
        assert!(error.contains("take_external"));
        assert!(error.contains("keep_local"));
    }
}
