//! Host-owned edit proposal store and safe workspace writes for review Apply/Deny.
//!
//! Proposals hold planned file mutations until the user applies or denies them.
//! Apply writes under the workspace root only (no path escape). The Webview never
//! receives secrets; it only sends path/content already visible in chat diffs.

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde_json::{json, Map, Value};

const MAX_PROPOSALS: usize = 256;
const MAX_PROPOSAL_ID_LEN: usize = 256;
const MAX_PATH_LEN: usize = 4096;
const MAX_CONTENT_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProposalKind {
    EditFile,
    CreateFile,
    CreateDirectory,
}

impl ProposalKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::EditFile => "edit_file",
            Self::CreateFile => "create_file",
            Self::CreateDirectory => "create_directory",
        }
    }

    fn parse(raw: Option<&str>) -> Result<Self, &'static str> {
        match raw.unwrap_or("edit_file") {
            "edit_file" | "edit" | "write_file" | "multi_edit" => Ok(Self::EditFile),
            "create_file" => Ok(Self::CreateFile),
            "create_directory" => Ok(Self::CreateDirectory),
            _ => Err("invalid_proposal_kind"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct EditProposal {
    pub id: String,
    pub path: String,
    pub kind: ProposalKind,
    pub original_content: String,
    pub proposed_content: String,
    pub chat_id: Option<String>,
    pub run_id: Option<String>,
    pub applied: bool,
}

impl EditProposal {
    pub fn to_json(&self) -> Value {
        json!({
            "id": self.id,
            "path": self.path,
            "kind": self.kind.as_str(),
            "original_content": self.original_content,
            "proposed_content": self.proposed_content,
            "chat_id": self.chat_id,
            "run_id": self.run_id,
            "applied": self.applied,
            "is_new_file": self.kind == ProposalKind::CreateFile
                || (self.original_content.is_empty()
                    && !self.proposed_content.is_empty()
                    && self.kind != ProposalKind::CreateDirectory),
        })
    }
}

#[derive(Debug, Default)]
pub struct EditProposalStore {
    by_id: HashMap<String, EditProposal>,
    /// Successful applies — reject duplicate apply with already_applied.
    applied_ids: HashMap<String, String>,
}

pub type SharedEditProposalStore = Arc<Mutex<EditProposalStore>>;

pub fn new_shared_store() -> SharedEditProposalStore {
    Arc::new(Mutex::new(EditProposalStore::default()))
}

impl EditProposalStore {
    pub fn upsert(&mut self, proposal: EditProposal) -> Result<(), &'static str> {
        if proposal.applied {
            return Err("invalid_proposal_state");
        }
        if !self.by_id.contains_key(&proposal.id) && self.by_id.len() >= MAX_PROPOSALS {
            return Err("proposal_store_full");
        }
        self.by_id.insert(proposal.id.clone(), proposal);
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&EditProposal> {
        self.by_id.get(id)
    }

    pub fn remove(&mut self, id: &str) -> Option<EditProposal> {
        self.by_id.remove(id)
    }

    pub fn list(&self, chat_id: Option<&str>) -> Vec<&EditProposal> {
        let mut items: Vec<&EditProposal> = self
            .by_id
            .values()
            .filter(|p| !p.applied)
            .filter(|p| match chat_id {
                Some(want) if !want.is_empty() => p.chat_id.as_deref() == Some(want),
                _ => true,
            })
            .collect();
        items.sort_by(|a, b| a.id.cmp(&b.id));
        items
    }

    pub fn was_applied(&self, id: &str) -> bool {
        self.applied_ids.contains_key(id)
    }

    pub fn mark_applied(&mut self, id: &str, path: &str) {
        self.applied_ids.insert(id.to_string(), path.to_string());
        self.by_id.remove(id);
    }
}

pub fn validate_proposal_id(id: &str) -> Result<&str, &'static str> {
    let id = id.trim();
    if id.is_empty()
        || id.len() > MAX_PROPOSAL_ID_LEN
        || id.contains('/')
        || id.contains('\\')
        || id.contains("..")
    {
        return Err("invalid_proposal_id");
    }
    Ok(id)
}

pub fn validate_content(content: &str) -> Result<(), &'static str> {
    if content.len() > MAX_CONTENT_BYTES {
        return Err("proposal_content_too_large");
    }
    Ok(())
}

/// Resolve `path` under `workspace_root`. Rejects empty paths, parent escape, and
/// absolute paths outside the root.
pub fn resolve_workspace_path(workspace_root: &Path, path: &str) -> Result<PathBuf, &'static str> {
    let path = path.trim();
    if path.is_empty() || path.len() > MAX_PATH_LEN {
        return Err("invalid_proposal_path");
    }
    if path.contains('\0') {
        return Err("invalid_proposal_path");
    }

    let candidate = if Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else {
        // Drop any `..` / `.` components before join to reject escapes early.
        let mut cleaned = PathBuf::new();
        for component in Path::new(path).components() {
            match component {
                Component::Normal(part) => cleaned.push(part),
                Component::CurDir => {}
                Component::ParentDir => return Err("path_outside_workspace"),
                Component::RootDir | Component::Prefix(_) => return Err("invalid_proposal_path"),
            }
        }
        if cleaned.as_os_str().is_empty() {
            return Err("invalid_proposal_path");
        }
        workspace_root.join(cleaned)
    };

    let root = canonicalize_existing(workspace_root).map_err(|_| "workspace_unavailable")?;
    let resolved = if candidate.exists() {
        canonicalize_existing(&candidate).map_err(|_| "invalid_proposal_path")?
    } else {
        // Target does not exist yet: canonicalize parent, append basename.
        let parent = candidate
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(workspace_root);
        let file_name = candidate
            .file_name()
            .ok_or("invalid_proposal_path")?;
        let parent_canon = if parent.exists() {
            canonicalize_existing(parent).map_err(|_| "invalid_proposal_path")?
        } else {
            // Intermediate dirs may be missing for create_file; walk up until one exists.
            let mut cursor = parent.to_path_buf();
            let mut missing = Vec::new();
            while !cursor.exists() {
                let name = cursor
                    .file_name()
                    .ok_or("invalid_proposal_path")?
                    .to_os_string();
                missing.push(name);
                cursor = cursor
                    .parent()
                    .filter(|p| !p.as_os_str().is_empty())
                    .ok_or("invalid_proposal_path")?
                    .to_path_buf();
            }
            let mut base = canonicalize_existing(&cursor).map_err(|_| "invalid_proposal_path")?;
            for part in missing.into_iter().rev() {
                base.push(part);
            }
            base.push(file_name);
            return ensure_under_root(&root, &base);
        };
        let joined = parent_canon.join(file_name);
        return ensure_under_root(&root, &joined);
    };

    ensure_under_root(&root, &resolved)
}

fn ensure_under_root(root: &Path, path: &Path) -> Result<PathBuf, &'static str> {
    if path == root || path.starts_with(root) {
        Ok(path.to_path_buf())
    } else {
        Err("path_outside_workspace")
    }
}

fn canonicalize_existing(path: &Path) -> Result<PathBuf, std::io::Error> {
    fs::canonicalize(path)
}

pub fn proposal_from_params(params: &Map<String, Value>) -> Result<EditProposal, &'static str> {
    let id = validate_proposal_id(params.get("id").and_then(Value::as_str).unwrap_or(""))?;
    let path = params
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    if path.is_empty() || path.len() > MAX_PATH_LEN {
        return Err("invalid_proposal_path");
    }
    let kind = ProposalKind::parse(params.get("kind").and_then(Value::as_str))?;
    let original_content = params
        .get("original_content")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let proposed_content = params
        .get("proposed_content")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    validate_content(&original_content)?;
    validate_content(&proposed_content)?;
    if kind != ProposalKind::CreateDirectory && proposed_content.is_empty() && original_content.is_empty()
    {
        // Allow empty write to clear a file, but reject empty create with no body
        // when kind is create without content — still ok for truncate edits.
    }
    let chat_id = params
        .get("chat_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let run_id = params
        .get("run_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    Ok(EditProposal {
        id: id.to_string(),
        path: path.to_string(),
        kind,
        original_content,
        proposed_content,
        chat_id,
        run_id,
        applied: false,
    })
}

pub fn apply_proposal_to_disk(
    workspace_root: &Path,
    proposal: &EditProposal,
) -> Result<(), String> {
    let target = resolve_workspace_path(workspace_root, &proposal.path).map_err(|e| e.to_string())?;
    match proposal.kind {
        ProposalKind::CreateDirectory => {
            fs::create_dir_all(&target).map_err(|e| format!("proposal_apply_failed:{e}"))?;
        }
        ProposalKind::EditFile | ProposalKind::CreateFile => {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|e| format!("proposal_apply_failed:{e}"))?;
            }
            write_atomic(&target, proposal.proposed_content.as_bytes())
                .map_err(|e| format!("proposal_apply_failed:{e}"))?;
        }
    }
    Ok(())
}

fn write_atomic(target: &Path, content: &[u8]) -> std::io::Result<()> {
    let parent = target.parent().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "path has no parent")
    })?;
    let temp_name = format!(
        ".altai-proposal-{}.tmp",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );
    let temp_path = parent.join(temp_name);
    {
        let mut file = fs::File::create(&temp_path)?;
        file.write_all(content)?;
        file.sync_all()?;
    }
    fs::rename(&temp_path, target).inspect_err(|_error| {
        let _ = fs::remove_file(&temp_path);
    })?;
    Ok(())
}

/// Upsert from params and return the stored JSON record.
pub fn handle_upsert(
    store: &SharedEditProposalStore,
    workspace_root: &Path,
    params: &Map<String, Value>,
) -> Result<Value, &'static str> {
    let mut proposal = proposal_from_params(params)?;
    // Validate path is inside workspace even before apply.
    resolve_workspace_path(workspace_root, &proposal.path)?;
    // Infer create_file when original empty and content present.
    if proposal.kind == ProposalKind::EditFile
        && proposal.original_content.is_empty()
        && !proposal.proposed_content.is_empty()
    {
        proposal.kind = ProposalKind::CreateFile;
    }
    let mut guard = store.lock().map_err(|_| "proposal_store_unavailable")?;
    if guard.was_applied(&proposal.id) {
        return Err("already_applied");
    }
    if let Some(existing) = guard.get(&proposal.id) {
        if existing.applied {
            return Err("already_applied");
        }
    }
    guard.upsert(proposal.clone())?;
    Ok(json!({ "proposal": proposal.to_json() }))
}

pub fn handle_list(
    store: &SharedEditProposalStore,
    params: &Map<String, Value>,
) -> Result<Value, &'static str> {
    let chat_id = params.get("chat_id").and_then(Value::as_str);
    let guard = store.lock().map_err(|_| "proposal_store_unavailable")?;
    let proposals: Vec<Value> = guard.list(chat_id).into_iter().map(|p| p.to_json()).collect();
    Ok(json!({ "proposals": proposals }))
}

/// Apply by id; optional body fields upsert/merge content before write.
pub fn handle_apply(
    store: &SharedEditProposalStore,
    workspace_root: &Path,
    params: &Map<String, Value>,
) -> Result<Value, String> {
    let id = validate_proposal_id(params.get("id").and_then(Value::as_str).unwrap_or(""))
        .map_err(|e| e.to_string())?;

    {
        let guard = store.lock().map_err(|_| "proposal_store_unavailable".to_string())?;
        if guard.was_applied(id) {
            return Err("already_applied".to_string());
        }
        if let Some(existing) = guard.get(id) {
            if existing.applied {
                return Err("already_applied".to_string());
            }
        }
    }

    // If path/content supplied, upsert (or refresh) before apply.
    if params.get("path").and_then(Value::as_str).is_some()
        || params.get("proposed_content").and_then(Value::as_str).is_some()
    {
        handle_upsert(store, workspace_root, params).map_err(|e| e.to_string())?;
    }

    let proposal = {
        let mut guard = store
            .lock()
            .map_err(|_| "proposal_store_unavailable".to_string())?;
        if guard.was_applied(id) {
            return Err("already_applied".to_string());
        }
        let proposal = guard
            .get(id)
            .cloned()
            .ok_or_else(|| "unknown_proposal".to_string())?;
        if proposal.applied {
            return Err("already_applied".to_string());
        }
        apply_proposal_to_disk(workspace_root, &proposal)?;
        guard.mark_applied(id, &proposal.path);
        proposal
    };

    Ok(json!({
        "applied": true,
        "id": proposal.id,
        "path": proposal.path,
    }))
}

pub fn handle_deny(
    store: &SharedEditProposalStore,
    params: &Map<String, Value>,
) -> Result<Value, &'static str> {
    let id = validate_proposal_id(params.get("id").and_then(Value::as_str).unwrap_or(""))?;
    let mut guard = store.lock().map_err(|_| "proposal_store_unavailable")?;
    // Idempotent: missing or already gone is success.
    guard.remove(id);
    Ok(json!({ "denied": true, "id": id }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn rejects_path_escape() {
        let dir = tempdir().unwrap();
        let err = resolve_workspace_path(dir.path(), "../outside.txt").unwrap_err();
        assert_eq!(err, "path_outside_workspace");
    }

    #[test]
    fn resolves_relative_inside_workspace() {
        let dir = tempdir().unwrap();
        let root = fs::canonicalize(dir.path()).unwrap();
        let target = resolve_workspace_path(&root, "src/main.rs").unwrap();
        assert!(target.starts_with(&root));
        assert!(target.ends_with("main.rs"));
    }

    #[test]
    fn apply_and_deny_round_trip() {
        let dir = tempdir().unwrap();
        let store = new_shared_store();
        let mut params = Map::new();
        params.insert("id".into(), json!("p1"));
        params.insert("path".into(), json!("note.txt"));
        params.insert("proposed_content".into(), json!("hello"));
        params.insert("kind".into(), json!("create_file"));

        handle_upsert(&store, dir.path(), &params).unwrap();
        let applied = handle_apply(&store, dir.path(), &params).unwrap();
        assert_eq!(applied["applied"], true);
        assert_eq!(fs::read_to_string(dir.path().join("note.txt")).unwrap(), "hello");

        // Second apply of same id rejected even with one-shot body
        let err = handle_apply(&store, dir.path(), &params).unwrap_err();
        assert_eq!(err, "already_applied");

        // Deny is idempotent
        let denied = handle_deny(&store, &params).unwrap();
        assert_eq!(denied["denied"], true);
    }

    #[test]
    fn apply_one_shot_without_prior_upsert() {
        let dir = tempdir().unwrap();
        let store = new_shared_store();
        let mut params = Map::new();
        params.insert("id".into(), json!("oneshot"));
        params.insert("path".into(), json!("a/b.txt"));
        params.insert("proposed_content".into(), json!("x"));
        let result = handle_apply(&store, dir.path(), &params).unwrap();
        assert_eq!(result["applied"], true);
        assert_eq!(
            fs::read_to_string(dir.path().join("a").join("b.txt")).unwrap(),
            "x"
        );
    }
}
