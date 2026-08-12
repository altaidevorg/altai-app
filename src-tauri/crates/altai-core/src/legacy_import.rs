//! Read-only preview of legacy project records that may later become Work.
//!
//! This module deliberately has no import/apply API and is the core half of a
//! backend-only dry run. It never opens a source database with write flags and
//! never creates a missing source or Work store. Every present SQLite source is
//! copied from identity-bound read handles into a private temporary snapshot;
//! SQLite may write only inside that temp directory, which is removed after
//! preview. User-visible preview reporting is intentionally planned as the
//! immediately following UI slice.

use rusqlite::{Connection, OpenFlags, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const MAX_LEGACY_JSON_BYTES: u64 = 4 * 1024 * 1024;
const MAX_JSON_SOURCE_ITEMS: usize = 2_000;
const MAX_TASK_RUN_RECORDS: i64 = 5_000;
const MAX_MARKER_RECORDS: i64 = 10_000;
const MAX_PREVIEW_OUTPUT_ITEMS: usize = 10_000;
const MAX_SQLITE_TEXT_BYTES: i64 = 256 * 1024;
const MAX_MARKER_PAYLOAD_BYTES: i64 = 4 * 1024;
const MAX_SQLITE_FILE_BYTES: u64 = 512 * 1024 * 1024;
const MAX_SQLITE_SIDECAR_BYTES: u64 = 512 * 1024 * 1024;
const MAX_SQLITE_DECODED_BYTES: usize = 16 * 1024 * 1024;
const MAX_ID_BYTES: usize = 4_096;
const MAX_TITLE_BYTES: usize = 16_384;
const SQLITE_SNAPSHOT_ATTEMPTS: usize = 3;

/// Event marker reserved for the future apply step. Preview uses this exact
/// pair to detect records already imported; it never writes the marker.
pub const LEGACY_IMPORT_EVENT_KIND: &str = "legacy_imported";
pub const LEGACY_IMPORT_SOURCE_KEY_FIELD: &str = "legacySourceKey";
/// Keeps even worst-case JSON escaping plus the marker field envelope below
/// the 4 KiB Work-event payload limit.
pub const LEGACY_IMPORT_SOURCE_KEY_MAX_BYTES: usize = 512;

fn framed_source_key(kind: &str, components: &[&str]) -> String {
    let mut key = format!("legacy:v1:{kind}");
    for component in components {
        key.push(':');
        key.push_str(&component.len().to_string());
        key.push(':');
        key.push_str(component);
    }
    key
}

pub fn legacy_assignment_source_key(id: &str) -> String {
    framed_source_key("assignment", &[id])
}

pub fn legacy_todo_source_key(session_id: &str, todo_id: &str) -> String {
    framed_source_key("todo", &[session_id, todo_id])
}

pub fn legacy_task_run_source_key(chat_id: &str) -> String {
    framed_source_key("task-run", &[chat_id])
}

fn checked_source_key(key: String) -> Option<String> {
    (key.len() <= LEGACY_IMPORT_SOURCE_KEY_MAX_BYTES).then_some(key)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacySourceIdentity {
    len: u64,
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
    #[cfg(windows)]
    volume_serial: u32,
    #[cfg(windows)]
    file_index: u64,
    #[cfg(not(any(unix, windows)))]
    modified: Option<std::time::SystemTime>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyWorkspaceIdentity {
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
    #[cfg(windows)]
    volume_serial: u32,
    #[cfg(windows)]
    file_index: u64,
    #[cfg(not(any(unix, windows)))]
    modified: Option<std::time::SystemTime>,
}

impl LegacyWorkspaceIdentity {
    #[cfg(not(windows))]
    fn from_metadata(metadata: &fs::Metadata) -> Option<Self> {
        if !metadata.is_dir() {
            return None;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            Some(Self {
                device: metadata.dev(),
                inode: metadata.ino(),
            })
        }
        #[cfg(not(any(unix, windows)))]
        {
            Some(Self {
                modified: metadata.modified().ok(),
            })
        }
    }

    #[cfg(windows)]
    fn from_path(path: &Path) -> Option<Self> {
        let file = open_windows_identity_handle(path, true)?;
        let (volume_serial, file_index) = windows_file_identity(&file, true)?;
        Some(Self {
            volume_serial,
            file_index,
        })
    }

    fn matches_path(&self, path: &Path, metadata: &fs::Metadata) -> bool {
        #[cfg(windows)]
        {
            let _ = metadata;
            Self::from_path(path).as_ref() == Some(self)
        }
        #[cfg(not(windows))]
        {
            let _ = path;
            Self::from_metadata(metadata).as_ref() == Some(self)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyWorkspaceRoot {
    pub path: PathBuf,
    expected_identity: LegacyWorkspaceIdentity,
}

impl LegacyWorkspaceRoot {
    pub fn capture(path: PathBuf) -> Result<Self, String> {
        let path = path
            .canonicalize()
            .map_err(|_error| "workspace_identity_unavailable".to_string())?;
        let metadata = path
            .symlink_metadata()
            .map_err(|_error| "workspace_identity_unavailable".to_string())?;
        if metadata.file_type().is_symlink() {
            return Err("workspace_identity_changed".into());
        }
        #[cfg(windows)]
        let expected_identity = LegacyWorkspaceIdentity::from_path(&path)
            .ok_or_else(|| "workspace_identity_changed".to_string())?;
        #[cfg(not(windows))]
        let expected_identity = LegacyWorkspaceIdentity::from_metadata(&metadata)
            .ok_or_else(|| "workspace_identity_changed".to_string())?;
        Ok(Self {
            path,
            expected_identity,
        })
    }

    pub fn is_current(&self) -> bool {
        self.path.symlink_metadata().is_ok_and(|metadata| {
            !metadata.file_type().is_symlink()
                && self.expected_identity.matches_path(&self.path, &metadata)
        })
    }
}

impl LegacySourceIdentity {
    #[cfg(not(windows))]
    pub fn from_metadata(metadata: &fs::Metadata) -> Option<Self> {
        if !metadata.is_file() {
            return None;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            Some(Self {
                len: metadata.len(),
                device: metadata.dev(),
                inode: metadata.ino(),
            })
        }
        #[cfg(not(any(unix, windows)))]
        {
            Some(Self {
                len: metadata.len(),
                modified: metadata.modified().ok(),
            })
        }
    }

    #[cfg(windows)]
    fn from_file(file: &fs::File, metadata: &fs::Metadata) -> Option<Self> {
        let (volume_serial, file_index) = windows_file_identity(file, false)?;
        Some(Self {
            len: metadata.len(),
            volume_serial,
            file_index,
        })
    }

    fn matches_file(&self, file: &fs::File, metadata: &fs::Metadata) -> bool {
        #[cfg(windows)]
        {
            Self::from_file(file, metadata).as_ref() == Some(self)
        }
        #[cfg(not(windows))]
        {
            let _ = file;
            Self::from_metadata(metadata).as_ref() == Some(self)
        }
    }

    /// Revalidates this captured identity against the current path.
    ///
    /// Windows reopens `path` with `FILE_FLAG_OPEN_REPARSE_POINT` and rejects
    /// every reparse-point type before comparing handle identity. Other
    /// platforms compare the supplied no-follow metadata identity.
    pub fn matches_path(&self, path: &Path, metadata: &fs::Metadata) -> bool {
        #[cfg(windows)]
        {
            let _ = metadata;
            open_source_file_no_follow(path)
                .ok()
                .and_then(|file| {
                    let opened = file.metadata().ok()?;
                    Self::from_file(&file, &opened)
                })
                .as_ref()
                == Some(self)
        }
        #[cfg(not(windows))]
        {
            let _ = path;
            Self::from_metadata(metadata).as_ref() == Some(self)
        }
    }

    fn len(&self) -> u64 {
        self.len
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyImportSource {
    pub path: PathBuf,
    pub expected_identity: Option<LegacySourceIdentity>,
}

impl LegacyImportSource {
    pub fn missing(path: PathBuf) -> Self {
        Self {
            path,
            expected_identity: None,
        }
    }

    pub fn capture(path: PathBuf) -> Result<Self, String> {
        let metadata = fs::symlink_metadata(&path)
            .map_err(|_error| "legacy_source_inspection_failed".to_string())?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err("legacy_source_type_rejected".into());
        }
        let file = open_source_file_no_follow(&path)?;
        let opened = file
            .metadata()
            .map_err(|_error| "legacy_source_identity_unavailable".to_string())?;
        #[cfg(windows)]
        let identity = LegacySourceIdentity::from_file(&file, &opened)
            .ok_or_else(|| "legacy_source_type_rejected".to_string())?;
        #[cfg(not(windows))]
        let identity = LegacySourceIdentity::from_metadata(&opened)
            .ok_or_else(|| "legacy_source_type_rejected".to_string())?;
        #[cfg(not(windows))]
        if !identity.matches_path(&path, &metadata) {
            return Err("legacy_source_identity_changed".into());
        }
        Ok(Self {
            path,
            expected_identity: Some(identity),
        })
    }
}

impl AsRef<Path> for LegacyImportSource {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

impl std::ops::Deref for LegacyImportSource {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &self.path
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacySqliteSource {
    pub main: LegacyImportSource,
    pub wal: Option<LegacyImportSource>,
    pub shm: Option<LegacyImportSource>,
}

impl LegacySqliteSource {
    pub fn missing(path: PathBuf) -> Self {
        Self {
            main: LegacyImportSource::missing(path),
            wal: None,
            shm: None,
        }
    }

    pub fn capture(path: PathBuf) -> Result<Self, String> {
        let main = LegacyImportSource::capture(path.clone())?;
        let wal_path = sqlite_sidecar_path(&path, "-wal");
        let shm_path = sqlite_sidecar_path(&path, "-shm");
        let wal = capture_optional_source(wal_path)?;
        let shm = capture_optional_source(shm_path)?;
        if wal.is_some() != shm.is_some() {
            return Err("sqlite_wal_pair_incomplete".into());
        }
        Ok(Self { main, wal, shm })
    }

    fn has_wal_pair(&self) -> bool {
        self.wal.is_some() && self.shm.is_some()
    }
}

impl AsRef<Path> for LegacySqliteSource {
    fn as_ref(&self) -> &Path {
        &self.main.path
    }
}

impl std::ops::Deref for LegacySqliteSource {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &self.main.path
    }
}

fn sqlite_sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(suffix);
    PathBuf::from(value)
}

fn capture_optional_source(path: PathBuf) -> Result<Option<LegacyImportSource>, String> {
    match fs::symlink_metadata(&path) {
        Ok(_) => LegacyImportSource::capture(path).map(Some),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_error) => Err("legacy_source_inspection_failed".into()),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyImportPreviewPaths {
    pub selected_workspace: LegacyWorkspaceRoot,
    pub assignments_json: LegacyImportSource,
    pub todos_json: LegacyImportSource,
    pub task_run_journal_db: LegacySqliteSource,
    pub work_db: LegacySqliteSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LegacyImportSourceKind {
    Assignment,
    ManualTodo,
    TaskRun,
    WorkDedupe,
}

impl LegacyImportSourceKind {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Assignment => "assignment",
            Self::ManualTodo => "manual_todo",
            Self::TaskRun => "task_run",
            Self::WorkDedupe => "work_dedupe",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LegacyImportDisposition {
    Candidate,
    Skipped,
    Duplicate,
    Error,
}

impl LegacyImportDisposition {
    fn as_str(self) -> &'static str {
        match self {
            Self::Candidate => "candidate",
            Self::Skipped => "skipped",
            Self::Duplicate => "duplicate",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyImportPreviewItem {
    pub source_kind: LegacyImportSourceKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proposed_work_state: Option<String>,
    pub disposition: LegacyImportDisposition,
    pub reason: String,
}

impl LegacyImportPreviewItem {
    fn candidate(
        source_kind: LegacyImportSourceKind,
        source_key: String,
        title: String,
        proposed_work_state: &str,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            source_kind,
            source_key: Some(source_key),
            title: Some(title),
            proposed_work_state: Some(proposed_work_state.to_string()),
            disposition: LegacyImportDisposition::Candidate,
            reason: reason.into(),
        }
    }

    fn skipped(
        source_kind: LegacyImportSourceKind,
        source_key: Option<String>,
        title: Option<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            source_kind,
            source_key,
            title,
            proposed_work_state: None,
            disposition: LegacyImportDisposition::Skipped,
            reason: reason.into(),
        }
    }

    fn error(source_kind: LegacyImportSourceKind, reason: impl Into<String>) -> Self {
        Self {
            source_kind,
            source_key: None,
            title: None,
            proposed_work_state: None,
            disposition: LegacyImportDisposition::Error,
            reason: reason.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyImportPreviewReport {
    pub candidate_count: usize,
    pub skipped_count: usize,
    pub duplicate_count: usize,
    pub error_count: usize,
    pub items: Vec<LegacyImportPreviewItem>,
}

impl LegacyImportPreviewReport {
    fn from_items(mut items: Vec<LegacyImportPreviewItem>) -> Self {
        items.sort_by(|left, right| {
            (
                left.source_key.as_deref().unwrap_or(""),
                left.source_kind.as_str(),
                left.disposition.as_str(),
                left.title.as_deref().unwrap_or(""),
                left.reason.as_str(),
            )
                .cmp(&(
                    right.source_key.as_deref().unwrap_or(""),
                    right.source_kind.as_str(),
                    right.disposition.as_str(),
                    right.title.as_deref().unwrap_or(""),
                    right.reason.as_str(),
                ))
        });
        Self {
            candidate_count: count_disposition(&items, LegacyImportDisposition::Candidate),
            skipped_count: count_disposition(&items, LegacyImportDisposition::Skipped),
            duplicate_count: count_disposition(&items, LegacyImportDisposition::Duplicate),
            error_count: count_disposition(&items, LegacyImportDisposition::Error),
            items,
        }
    }
}

fn count_disposition(
    items: &[LegacyImportPreviewItem],
    disposition: LegacyImportDisposition,
) -> usize {
    items
        .iter()
        .filter(|item| item.disposition == disposition)
        .count()
}

/// Build the complete preview. Missing legacy files and a missing `work.db`
/// are valid empty inputs. Any present source is opened read-only.
pub fn preview_legacy_import(paths: &LegacyImportPreviewPaths) -> LegacyImportPreviewReport {
    if !paths.selected_workspace.is_current() {
        return workspace_identity_error_report();
    }
    let mut items = Vec::new();
    read_json_source(
        &paths.assignments_json,
        LegacyImportSourceKind::Assignment,
        parse_legacy_assignments_for_root,
        &paths.selected_workspace,
        &mut items,
    );
    read_json_source(
        &paths.todos_json,
        LegacyImportSourceKind::ManualTodo,
        parse_legacy_todos_for_root,
        &paths.selected_workspace,
        &mut items,
    );

    match paths.task_run_journal_db.main.expected_identity.as_ref() {
        None => {}
        Some(_) => match read_task_run_candidates(&paths.task_run_journal_db) {
            Ok(mut task_runs) => items.append(&mut task_runs),
            Err(error) => items.push(LegacyImportPreviewItem::error(
                LegacyImportSourceKind::TaskRun,
                error,
            )),
        },
    }

    let existing_markers = match paths.work_db.main.expected_identity.as_ref() {
        None => Some(HashSet::new()),
        Some(_) => match read_existing_source_markers(&paths.work_db) {
            Ok(markers) => Some(markers),
            Err(error) => {
                items.push(LegacyImportPreviewItem::error(
                    LegacyImportSourceKind::WorkDedupe,
                    error,
                ));
                None
            }
        },
    };

    if items.len() > MAX_PREVIEW_OUTPUT_ITEMS {
        items.clear();
        items.push(LegacyImportPreviewItem::error(
            LegacyImportSourceKind::WorkDedupe,
            "legacy preview output exceeds the safe record limit",
        ));
    } else {
        classify_duplicates(&mut items, existing_markers.as_ref());
    }
    if !paths.selected_workspace.is_current() {
        return workspace_identity_error_report();
    }
    LegacyImportPreviewReport::from_items(items)
}

fn workspace_identity_error_report() -> LegacyImportPreviewReport {
    LegacyImportPreviewReport::from_items(vec![LegacyImportPreviewItem::error(
        LegacyImportSourceKind::WorkDedupe,
        "workspace_identity_changed_during_preview",
    )])
}

fn read_json_source(
    source: &LegacyImportSource,
    source_kind: LegacyImportSourceKind,
    parser: fn(&[u8], &LegacyWorkspaceRoot) -> Vec<LegacyImportPreviewItem>,
    selected_workspace: &LegacyWorkspaceRoot,
    items: &mut Vec<LegacyImportPreviewItem>,
) {
    let Some(expected_identity) = source.expected_identity.as_ref() else {
        return;
    };
    if expected_identity.len() > MAX_LEGACY_JSON_BYTES {
        items.push(LegacyImportPreviewItem::error(
            source_kind,
            format!("legacy JSON source exceeds the {MAX_LEGACY_JSON_BYTES}-byte preview limit"),
        ));
        return;
    }
    let mut file = match open_source_file_no_follow(&source.path) {
        Ok(file) => file,
        Err(reason) => {
            items.push(LegacyImportPreviewItem::error(source_kind, reason));
            return;
        }
    };
    let opened = match file.metadata() {
        Ok(metadata) => metadata,
        Err(_error) => {
            items.push(LegacyImportPreviewItem::error(
                source_kind,
                "legacy_source_identity_unavailable",
            ));
            return;
        }
    };
    if !expected_identity.matches_file(&file, &opened) {
        items.push(LegacyImportPreviewItem::error(
            source_kind,
            "legacy_source_identity_changed",
        ));
        return;
    }
    let mut first_read = Vec::new();
    match Read::by_ref(&mut file)
        .take(MAX_LEGACY_JSON_BYTES + 1)
        .read_to_end(&mut first_read)
    {
        Ok(_) if first_read.len() as u64 > MAX_LEGACY_JSON_BYTES => {
            items.push(LegacyImportPreviewItem::error(
                source_kind,
                format!(
                    "legacy JSON source exceeds the {MAX_LEGACY_JSON_BYTES}-byte preview limit"
                ),
            ));
        }
        Ok(_) => {
            if file.rewind().is_err() {
                items.push(LegacyImportPreviewItem::error(
                    source_kind,
                    "legacy_source_read_failed",
                ));
                return;
            }
            let mut second_read = Vec::with_capacity(first_read.len());
            if Read::by_ref(&mut file)
                .take(MAX_LEGACY_JSON_BYTES + 1)
                .read_to_end(&mut second_read)
                .is_err()
            {
                items.push(LegacyImportPreviewItem::error(
                    source_kind,
                    "legacy_source_read_failed",
                ));
                return;
            }
            if first_read != second_read {
                items.push(LegacyImportPreviewItem::error(
                    source_kind,
                    "legacy_source_changed_during_read",
                ));
                return;
            }
            items.extend(parser(&first_read, selected_workspace));
        }
        Err(_error) => items.push(LegacyImportPreviewItem::error(
            source_kind,
            "legacy_source_read_failed",
        )),
    }
}

fn open_source_file_no_follow(path: &Path) -> Result<fs::File, String> {
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
        };
        options
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
    let file = options
        .open(path)
        .map_err(|_error| "legacy_source_open_failed".to_string())?;
    #[cfg(windows)]
    if windows_file_identity(&file, false).is_none() {
        return Err("legacy_source_type_rejected".into());
    }
    #[cfg(not(windows))]
    if file
        .metadata()
        .map_err(|_error| "legacy_source_identity_unavailable".to_string())?
        .file_type()
        .is_symlink()
    {
        return Err("legacy_source_type_rejected".into());
    }
    Ok(file)
}

#[cfg(windows)]
fn open_windows_identity_handle(path: &Path, directory: bool) -> Option<fs::File> {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_DELETE,
        FILE_SHARE_READ, FILE_SHARE_WRITE,
    };

    let mut options = fs::OpenOptions::new();
    options
        .access_mode(0)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .custom_flags(
            FILE_FLAG_OPEN_REPARSE_POINT
                | if directory {
                    FILE_FLAG_BACKUP_SEMANTICS
                } else {
                    0
                },
        );
    options.open(path).ok()
}

#[cfg(windows)]
fn windows_file_identity(file: &fs::File, directory: bool) -> Option<(u32, u64)> {
    use std::mem::MaybeUninit;
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION, FILE_ATTRIBUTE_DIRECTORY,
        FILE_ATTRIBUTE_REPARSE_POINT,
    };

    let mut info = MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::zeroed();
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), info.as_mut_ptr()) } == 0 {
        return None;
    }
    let info = unsafe { info.assume_init() };
    let is_directory = info.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0;
    let is_reparse_point = info.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0;
    if is_reparse_point || is_directory != directory {
        return None;
    }
    let file_index = (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow);
    Some((info.dwVolumeSerialNumber, file_index))
}

fn valid_bounded_field(value: &str, max_bytes: usize) -> bool {
    let trimmed = value.trim();
    !trimmed.is_empty() && trimmed.len() <= max_bytes
}

fn classify_duplicates(
    items: &mut [LegacyImportPreviewItem],
    existing_markers: Option<&HashSet<String>>,
) {
    let mut seen = HashSet::new();
    let mut candidates = items
        .iter_mut()
        .filter(|item| item.disposition == LegacyImportDisposition::Candidate)
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        (
            left.source_key.as_deref().unwrap_or(""),
            left.title.as_deref().unwrap_or(""),
            left.reason.as_str(),
        )
            .cmp(&(
                right.source_key.as_deref().unwrap_or(""),
                right.title.as_deref().unwrap_or(""),
                right.reason.as_str(),
            ))
    });

    for item in candidates {
        let Some(source_key) = item.source_key.as_deref() else {
            item.disposition = LegacyImportDisposition::Error;
            item.reason = "candidate is missing its deterministic source key".to_string();
            item.proposed_work_state = None;
            continue;
        };
        let Some(existing_markers) = existing_markers else {
            item.disposition = LegacyImportDisposition::Error;
            item.reason = "could not verify whether this source was already imported".to_string();
            item.proposed_work_state = None;
            continue;
        };
        if existing_markers.contains(source_key) {
            item.disposition = LegacyImportDisposition::Duplicate;
            item.reason = "exact legacy source marker already exists in work_events".to_string();
            item.proposed_work_state = None;
        } else if !seen.insert(source_key.to_string()) {
            item.disposition = LegacyImportDisposition::Duplicate;
            item.reason = "legacy sources contain the same deterministic source key more than once"
                .to_string();
            item.proposed_work_state = None;
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct LegacyAssignmentsStore {
    #[serde(default)]
    pub assignments: Vec<Value>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum LegacyAssignmentSource {
    Issue {
        owner: String,
        repo: String,
        number: u64,
        url: String,
    },
    Pr {
        owner: String,
        repo: String,
        number: u64,
        url: String,
    },
    Todo {
        #[serde(rename = "todoId")]
        todo_id: String,
    },
    Task {
        prompt: String,
    },
}

impl LegacyAssignmentSource {
    fn todo_id(&self) -> Option<&str> {
        match self {
            Self::Todo { todo_id } => Some(todo_id),
            _ => None,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyAssignmentOrchestration {
    pub workspace_key: String,
    pub task_session_id: String,
    pub task_key: String,
    pub attempt: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyAssignmentRunConfig {
    pub workspace_path: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyAssignment {
    pub id: String,
    pub source: LegacyAssignmentSource,
    pub session_id: String,
    pub title: String,
    pub status: String,
    pub origin: Option<String>,
    pub orchestration: Option<LegacyAssignmentOrchestration>,
    pub run_config: Option<LegacyAssignmentRunConfig>,
    pub created_at: u64,
    pub updated_at: u64,
}

/// Pure parser for the real `altai-assignments.json` store shape.
pub fn parse_legacy_assignments(
    bytes: &[u8],
    selected_workspace: &Path,
) -> Vec<LegacyImportPreviewItem> {
    let Ok(selected_workspace) = LegacyWorkspaceRoot::capture(selected_workspace.to_path_buf())
    else {
        return workspace_identity_error_report().items;
    };
    parse_legacy_assignments_for_root(bytes, &selected_workspace)
}

fn parse_legacy_assignments_for_root(
    bytes: &[u8],
    selected_workspace: &LegacyWorkspaceRoot,
) -> Vec<LegacyImportPreviewItem> {
    let value: Value = match serde_json::from_slice(bytes) {
        Ok(value) => value,
        Err(_error) => {
            return vec![LegacyImportPreviewItem::error(
                LegacyImportSourceKind::Assignment,
                "altai-assignments.json is invalid JSON",
            )]
        }
    };
    let Some(object) = value.as_object() else {
        return vec![LegacyImportPreviewItem::error(
            LegacyImportSourceKind::Assignment,
            "altai-assignments.json root must be an object",
        )];
    };
    let Some(raw_assignments) = object.get("assignments") else {
        return Vec::new();
    };
    let Some(assignments) = raw_assignments.as_array() else {
        return vec![LegacyImportPreviewItem::error(
            LegacyImportSourceKind::Assignment,
            "altai-assignments.json assignments must be an array",
        )];
    };
    if assignments.len() > MAX_JSON_SOURCE_ITEMS {
        return vec![LegacyImportPreviewItem::error(
            LegacyImportSourceKind::Assignment,
            "assignment source exceeds the safe record limit",
        )];
    }

    assignments
        .iter()
        .enumerate()
        .map(
            |(index, raw)| match serde_json::from_value::<LegacyAssignment>(raw.clone()) {
                Ok(assignment) => map_assignment(assignment, selected_workspace, index),
                Err(_error) => LegacyImportPreviewItem::error(
                    LegacyImportSourceKind::Assignment,
                    format!("assignment at index {index} does not match the supported schema"),
                ),
            },
        )
        .collect()
}

fn canonical_workspace_matches(
    candidate: Option<&str>,
    selected_workspace: &LegacyWorkspaceRoot,
) -> bool {
    let Some(candidate) = candidate else {
        return false;
    };
    let path = Path::new(candidate.trim());
    path.is_absolute()
        && path.canonicalize().is_ok_and(|candidate| {
            candidate == selected_workspace.path
                && candidate.symlink_metadata().is_ok_and(|metadata| {
                    !metadata.file_type().is_symlink()
                        && selected_workspace
                            .expected_identity
                            .matches_path(&candidate, &metadata)
                })
        })
}

fn assignment_matches_workspace(
    assignment: &LegacyAssignment,
    selected_workspace: &LegacyWorkspaceRoot,
) -> bool {
    let run_path = assignment
        .run_config
        .as_ref()
        .and_then(|config| config.workspace_path.as_deref());
    let workspace_key = assignment
        .orchestration
        .as_ref()
        .map(|orchestration| orchestration.workspace_key.as_str());
    match (run_path, workspace_key) {
        (Some(run_path), Some(workspace_key)) => {
            valid_bounded_field(run_path, MAX_ID_BYTES)
                && valid_bounded_field(workspace_key, MAX_ID_BYTES)
                && canonical_workspace_matches(Some(run_path), selected_workspace)
                && canonical_workspace_matches(Some(workspace_key), selected_workspace)
        }
        (Some(run_path), None) => {
            valid_bounded_field(run_path, MAX_ID_BYTES)
                && canonical_workspace_matches(Some(run_path), selected_workspace)
        }
        (None, Some(workspace_key)) => {
            valid_bounded_field(workspace_key, MAX_ID_BYTES)
                && canonical_workspace_matches(Some(workspace_key), selected_workspace)
        }
        (None, None) => false,
    }
}

fn assignment_source_key(assignment: &LegacyAssignment) -> Option<String> {
    let Some(todo_id) = assignment.source.todo_id() else {
        return checked_source_key(legacy_assignment_source_key(assignment.id.trim()));
    };
    let orchestration = assignment.orchestration.as_ref()?;
    (valid_bounded_field(todo_id, MAX_ID_BYTES)
        && valid_bounded_field(&orchestration.task_session_id, MAX_ID_BYTES)
        && valid_bounded_field(&orchestration.task_key, MAX_ID_BYTES)
        && orchestration.task_key.trim() == todo_id.trim())
    .then(|| {
        legacy_todo_source_key(
            orchestration.task_session_id.trim(),
            orchestration.task_key.trim(),
        )
    })
    .and_then(checked_source_key)
}

fn map_assignment(
    assignment: LegacyAssignment,
    selected_workspace: &LegacyWorkspaceRoot,
    index: usize,
) -> LegacyImportPreviewItem {
    let id = assignment.id.trim();
    let title = assignment.title.trim();
    if !valid_bounded_field(id, MAX_ID_BYTES)
        || !valid_bounded_field(title, MAX_TITLE_BYTES)
        || !valid_bounded_field(&assignment.session_id, MAX_ID_BYTES)
    {
        return LegacyImportPreviewItem::error(
            LegacyImportSourceKind::Assignment,
            format!("assignment at index {index} has an invalid bounded field"),
        );
    }
    let Some(fallback_source_key) = checked_source_key(legacy_assignment_source_key(id)) else {
        return LegacyImportPreviewItem::error(
            LegacyImportSourceKind::Assignment,
            format!("assignment at index {index} source key exceeds the preview field limit"),
        );
    };
    if !assignment_matches_workspace(&assignment, selected_workspace) {
        return LegacyImportPreviewItem::skipped(
            LegacyImportSourceKind::Assignment,
            Some(fallback_source_key),
            Some(title.to_string()),
            "assignment has no proven provenance for the selected workspace",
        );
    }
    if !matches!(
        assignment.origin.as_deref(),
        None | Some("manual") | Some("orchestrator")
    ) {
        return LegacyImportPreviewItem::error(
            LegacyImportSourceKind::Assignment,
            format!("assignment at index {index} has an unsupported origin"),
        );
    }
    let Some(source_key) = assignment_source_key(&assignment) else {
        return LegacyImportPreviewItem::skipped(
            LegacyImportSourceKind::Assignment,
            Some(fallback_source_key),
            Some(title.to_string()),
            "assignment todo provenance is inconsistent",
        );
    };
    let (state, reason) = match assignment.status.as_str() {
        "dispatching" => ("in_progress", "legacy assignment was dispatching"),
        "running" => ("in_progress", "legacy assignment has an active run"),
        "awaiting-approval" => (
            "in_progress",
            "legacy assignment is waiting for human approval",
        ),
        "done" => (
            "in_review",
            "legacy terminal success requires human Accept or Return",
        ),
        "failed" => ("ready", "legacy attempt failed and can be retried"),
        "cancelled" => ("cancelled", "legacy assignment was cancelled"),
        _other => {
            return LegacyImportPreviewItem::error(
                LegacyImportSourceKind::Assignment,
                format!("assignment at index {index} has an unsupported status"),
            )
        }
    };
    LegacyImportPreviewItem::candidate(
        LegacyImportSourceKind::Assignment,
        source_key,
        title.to_string(),
        state,
        reason,
    )
}

#[derive(Debug, Deserialize)]
pub struct LegacyTodo {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub origin: Option<String>,
}

/// Pure parser for the real `altai-ai-todos.json` key/value store shape.
/// Global todo records do not carry an exact workspace path. All are reported
/// as skipped until a workspace-local join can prove their provenance.
pub fn parse_legacy_todos(
    bytes: &[u8],
    _selected_workspace: &Path,
) -> Vec<LegacyImportPreviewItem> {
    let Ok(selected_workspace) = LegacyWorkspaceRoot::capture(_selected_workspace.to_path_buf())
    else {
        return workspace_identity_error_report().items;
    };
    parse_legacy_todos_for_root(bytes, &selected_workspace)
}

fn parse_legacy_todos_for_root(
    bytes: &[u8],
    _selected_workspace: &LegacyWorkspaceRoot,
) -> Vec<LegacyImportPreviewItem> {
    let value: Value = match serde_json::from_slice(bytes) {
        Ok(value) => value,
        Err(_error) => {
            return vec![LegacyImportPreviewItem::error(
                LegacyImportSourceKind::ManualTodo,
                "altai-ai-todos.json is invalid JSON",
            )]
        }
    };
    let Some(object) = value.as_object() else {
        return vec![LegacyImportPreviewItem::error(
            LegacyImportSourceKind::ManualTodo,
            "altai-ai-todos.json root must be an object",
        )];
    };

    let mut entries = object.iter().collect::<Vec<_>>();
    entries.sort_by_key(|(key, _)| key.as_str());
    let mut total_items = 0usize;
    for (key, value) in &entries {
        if !key.starts_with("todos:") {
            continue;
        }
        let Some(array) = value.as_array() else {
            return vec![LegacyImportPreviewItem::error(
                LegacyImportSourceKind::ManualTodo,
                "todo source contains a non-array session record",
            )];
        };
        total_items = total_items.saturating_add(array.len());
        if total_items > MAX_JSON_SOURCE_ITEMS {
            return vec![LegacyImportPreviewItem::error(
                LegacyImportSourceKind::ManualTodo,
                "todo source exceeds the safe record limit",
            )];
        }
    }
    let mut items = Vec::new();
    for (key, raw_todos) in entries {
        let Some(session_id) = key.strip_prefix("todos:") else {
            continue;
        };
        if !valid_bounded_field(session_id, MAX_ID_BYTES) {
            items.push(LegacyImportPreviewItem::error(
                LegacyImportSourceKind::ManualTodo,
                "todo store session id is empty or exceeds the preview field limit",
            ));
            continue;
        }
        let todos = raw_todos
            .as_array()
            .expect("todo arrays were validated before expansion");
        for (index, raw) in todos.iter().enumerate() {
            match serde_json::from_value::<LegacyTodo>(raw.clone()) {
                Ok(todo) => items.push(map_todo(session_id, todo, index)),
                Err(_error) => items.push(LegacyImportPreviewItem::error(
                    LegacyImportSourceKind::ManualTodo,
                    format!("todo record at index {index} does not match the supported schema"),
                )),
            }
        }
    }
    items
}

fn map_todo(session_id: &str, todo: LegacyTodo, index: usize) -> LegacyImportPreviewItem {
    let id = todo.id.trim();
    let title = todo.title.trim();
    if !valid_bounded_field(id, MAX_ID_BYTES) || !valid_bounded_field(title, MAX_TITLE_BYTES) {
        return LegacyImportPreviewItem::error(
            LegacyImportSourceKind::ManualTodo,
            format!("todo record at index {index} has an invalid bounded field"),
        );
    }
    let Some(source_key) = checked_source_key(legacy_todo_source_key(session_id, id)) else {
        return LegacyImportPreviewItem::error(
            LegacyImportSourceKind::ManualTodo,
            format!("todo record at index {index} source key exceeds the preview field limit"),
        );
    };
    match todo.origin.as_deref() {
        Some("manual") => {}
        None | Some("agent") => {
            return LegacyImportPreviewItem::skipped(
                LegacyImportSourceKind::ManualTodo,
                Some(source_key),
                Some(title.to_string()),
                "agent or origin-less run-plan todo is not project Work",
            )
        }
        Some(_other) => {
            return LegacyImportPreviewItem::error(
                LegacyImportSourceKind::ManualTodo,
                format!("todo record at index {index} has an unsupported origin"),
            )
        }
    }
    LegacyImportPreviewItem::skipped(
        LegacyImportSourceKind::ManualTodo,
        Some(source_key),
        Some(title.to_string()),
        "manual todo has no proven provenance for the selected workspace",
    )
}

struct SqliteSnapshot {
    directory: PathBuf,
}

impl Drop for SqliteSnapshot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

struct ReadOnlySqlite {
    connection: Connection,
    _snapshot: SqliteSnapshot,
}

impl std::ops::Deref for ReadOnlySqlite {
    type Target = Connection;

    fn deref(&self) -> &Self::Target {
        &self.connection
    }
}

impl std::ops::DerefMut for ReadOnlySqlite {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.connection
    }
}

fn open_read_only(source: &LegacySqliteSource) -> Result<ReadOnlySqlite, String> {
    let expected = source
        .main
        .expected_identity
        .as_ref()
        .ok_or_else(|| "sqlite_source_missing".to_string())?;
    if expected.len() > MAX_SQLITE_FILE_BYTES {
        return Err("read-only SQLite source exceeds the safe file-size limit".into());
    }
    let before = source
        .main
        .path
        .symlink_metadata()
        .map_err(|_error| "sqlite_source_inspection_failed".to_string())?;
    if before.file_type().is_symlink() || !expected.matches_path(&source.main.path, &before) {
        return Err("sqlite_source_identity_changed".into());
    }
    validate_sqlite_sidecars(source)?;
    // Never let SQLite open a source database in place. Even a nominally
    // read-only open can interact with rollback/WAL sidecars. Copy only from
    // identity-bound handles into a stable private snapshot and permit SQLite
    // writes solely inside that temporary directory.
    let (snapshot_path, snapshot) = create_sqlite_snapshot(source)?;
    let snapshot_path = snapshot_path
        .canonicalize()
        .map_err(|_error| "sqlite_snapshot_resolution_failed".to_string())?;
    let connection = Connection::open_with_flags(
        snapshot_path,
        OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX
            | OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )
    .map_err(|_error| "sqlite_snapshot_open_failed".to_string())?;
    validate_sqlite_source_identity(source)?;
    connection
        .pragma_update(None, "query_only", "ON")
        .map_err(|_error| "could not enforce SQLite query-only mode".to_string())?;
    Ok(ReadOnlySqlite {
        connection,
        _snapshot: snapshot,
    })
}

static SQLITE_SNAPSHOT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn create_sqlite_snapshot(
    source: &LegacySqliteSource,
) -> Result<(PathBuf, SqliteSnapshot), String> {
    if !source.has_wal_pair() {
        for _attempt in 0..SQLITE_SNAPSHOT_ATTEMPTS {
            if let Some(snapshot) = try_create_main_only_sqlite_snapshot(source)? {
                return Ok(snapshot);
            }
        }
        return Err("sqlite_snapshot_source_changed".into());
    }
    let wal = source
        .wal
        .as_ref()
        .ok_or_else(|| "sqlite_wal_pair_incomplete".to_string())?;
    let shm = source
        .shm
        .as_ref()
        .ok_or_else(|| "sqlite_wal_pair_incomplete".to_string())?;
    let wal_len = wal
        .expected_identity
        .as_ref()
        .ok_or_else(|| "sqlite_wal_identity_changed".to_string())?
        .len();
    let shm_len = shm
        .expected_identity
        .as_ref()
        .ok_or_else(|| "sqlite_shm_identity_changed".to_string())?
        .len();
    if wal_len > MAX_SQLITE_SIDECAR_BYTES || shm_len > MAX_SQLITE_SIDECAR_BYTES {
        return Err("sqlite_sidecar_size_limit_exceeded".into());
    }
    for _attempt in 0..SQLITE_SNAPSHOT_ATTEMPTS {
        if let Some(snapshot) = try_create_coherent_sqlite_snapshot(source, wal, shm)? {
            return Ok(snapshot);
        }
    }
    Err("sqlite_snapshot_source_changed".into())
}

fn try_create_main_only_sqlite_snapshot(
    source: &LegacySqliteSource,
) -> Result<Option<(PathBuf, SqliteSnapshot)>, String> {
    validate_sqlite_sidecars(source)?;
    let mut main_handle = open_captured_source(&source.main, "sqlite_source_identity_changed")?;
    let before = hash_source_handle(&mut main_handle)?;
    let directory = create_private_snapshot_directory()?;
    let guard = SqliteSnapshot {
        directory: directory.clone(),
    };
    let main_path = directory.join("source.db");
    let copied = copy_source_handle(&mut main_handle, &main_path)?;
    let after = hash_source_handle(&mut main_handle)?;
    validate_sqlite_source_identity(source)?;
    if before != after || copied != before {
        return Ok(None);
    }
    Ok(Some((main_path, guard)))
}

fn try_create_coherent_sqlite_snapshot(
    source: &LegacySqliteSource,
    wal: &LegacyImportSource,
    shm: &LegacyImportSource,
) -> Result<Option<(PathBuf, SqliteSnapshot)>, String> {
    // A main/WAL pair is useful only when both files describe the same WAL
    // generation. Hash all three identity-bound source handles around the
    // copy, and verify the copied bytes against the first hashes. A writer,
    // checkpoint, same-length rewrite, or WAL-index generation change causes
    // a bounded retry instead of admitting a torn snapshot.
    let mut main_handle = open_captured_source(&source.main, "sqlite_source_identity_changed")?;
    let mut wal_handle = open_captured_source(wal, "sqlite_wal_identity_changed")?;
    let mut shm_handle = open_captured_source(shm, "sqlite_shm_identity_changed")?;
    let before = (
        hash_source_handle(&mut main_handle)?,
        hash_source_handle(&mut wal_handle)?,
        hash_source_handle(&mut shm_handle)?,
    );
    let directory = create_private_snapshot_directory()?;
    let guard = SqliteSnapshot {
        directory: directory.clone(),
    };
    let main_path = directory.join("source.db");
    let wal_path = directory.join("source.db-wal");
    let copied_main = copy_source_handle(&mut main_handle, &main_path)?;
    let copied_wal = copy_source_handle(&mut wal_handle, &wal_path)?;
    let after = (
        hash_source_handle(&mut main_handle)?,
        hash_source_handle(&mut wal_handle)?,
        hash_source_handle(&mut shm_handle)?,
    );
    validate_sqlite_source_identity(source)?;
    if before != after || copied_main != before.0 || copied_wal != before.1 {
        return Ok(None);
    }
    Ok(Some((main_path, guard)))
}

type SourceDigest = [u8; 32];

struct CapturedSourceHandle {
    file: fs::File,
    expected_len: u64,
}

fn open_captured_source(
    source: &LegacyImportSource,
    identity_error: &str,
) -> Result<CapturedSourceHandle, String> {
    let expected = source
        .expected_identity
        .as_ref()
        .ok_or_else(|| identity_error.to_string())?;
    let file = open_source_file_no_follow(&source.path)?;
    let opened = file
        .metadata()
        .map_err(|_error| identity_error.to_string())?;
    if !expected.matches_file(&file, &opened) {
        return Err(identity_error.to_string());
    }
    Ok(CapturedSourceHandle {
        file,
        expected_len: expected.len(),
    })
}

fn hash_source_handle(handle: &mut CapturedSourceHandle) -> Result<SourceDigest, String> {
    handle
        .file
        .rewind()
        .map_err(|_error| "sqlite_snapshot_read_failed".to_string())?;
    let mut reader = (&mut handle.file).take(handle.expected_len + 1);
    let mut hasher = Sha256::new();
    let mut total = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|_error| "sqlite_snapshot_read_failed".to_string())?;
        if read == 0 {
            break;
        }
        total = total.saturating_add(read as u64);
        hasher.update(&buffer[..read]);
    }
    if total != handle.expected_len {
        return Err("sqlite_snapshot_source_changed".into());
    }
    Ok(hasher.finalize().into())
}

fn create_private_snapshot_directory() -> Result<PathBuf, String> {
    let base = std::env::temp_dir();
    for _ in 0..32 {
        let sequence = SQLITE_SNAPSHOT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let name = format!("altai-legacy-preview-{}-{sequence}", std::process::id());
        let path = base.join(name);
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        match builder.create(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_error) => return Err("sqlite_snapshot_directory_failed".into()),
        }
    }
    Err("sqlite_snapshot_directory_failed".into())
}

fn copy_source_handle(
    handle: &mut CapturedSourceHandle,
    destination: &Path,
) -> Result<SourceDigest, String> {
    handle
        .file
        .rewind()
        .map_err(|_error| "sqlite_snapshot_read_failed".to_string())?;
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(|_error| "sqlite_snapshot_write_failed".to_string())?;
    let mut reader = (&mut handle.file).take(handle.expected_len + 1);
    let mut hasher = Sha256::new();
    let mut total = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|_error| "sqlite_snapshot_copy_failed".to_string())?;
        if read == 0 {
            break;
        }
        total = total.saturating_add(read as u64);
        hasher.update(&buffer[..read]);
        output
            .write_all(&buffer[..read])
            .map_err(|_error| "sqlite_snapshot_write_failed".to_string())?;
    }
    output
        .flush()
        .map_err(|_error| "sqlite_snapshot_write_failed".to_string())?;
    if output
        .metadata()
        .map_err(|_error| "sqlite_snapshot_write_failed".to_string())?
        .len()
        != handle.expected_len
        || total != handle.expected_len
    {
        return Err("sqlite_snapshot_source_changed".into());
    }
    Ok(hasher.finalize().into())
}

fn validate_source_identity(source: &LegacyImportSource, code: &str) -> Result<(), String> {
    let expected = source
        .expected_identity
        .as_ref()
        .ok_or_else(|| code.to_string())?;
    let metadata = source
        .path
        .symlink_metadata()
        .map_err(|_error| code.to_string())?;
    if metadata.file_type().is_symlink() || !expected.matches_path(&source.path, &metadata) {
        return Err(code.to_string());
    }
    Ok(())
}

fn validate_sqlite_sidecars(source: &LegacySqliteSource) -> Result<(), String> {
    require_absent_sqlite_sidecar(&source.main.path, "-journal")?;
    match (&source.wal, &source.shm) {
        (None, None) => {
            for suffix in ["-wal", "-shm"] {
                require_absent_sqlite_sidecar(&source.main.path, suffix)?;
            }
            Ok(())
        }
        (Some(wal), Some(shm)) => {
            validate_source_identity(wal, "sqlite_wal_identity_changed")?;
            validate_source_identity(shm, "sqlite_shm_identity_changed")
        }
        _ => Err("sqlite_wal_pair_incomplete".into()),
    }
}

fn require_absent_sqlite_sidecar(main: &Path, suffix: &str) -> Result<(), String> {
    match fs::symlink_metadata(sqlite_sidecar_path(main, suffix)) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Ok(_) => Err(if suffix == "-journal" {
            "sqlite_rollback_journal_rejected"
        } else {
            "sqlite_sidecar_state_changed"
        }
        .to_string()),
        Err(_error) => Err("sqlite_sidecar_inspection_failed".into()),
    }
}

fn validate_sqlite_source_identity(source: &LegacySqliteSource) -> Result<(), String> {
    validate_source_identity(&source.main, "sqlite_source_identity_changed")?;
    validate_sqlite_sidecars(source)
}

fn validate_sqlite_bounds(connection: &Connection) -> Result<(), String> {
    let page_count: i64 = connection
        .pragma_query_value(None, "page_count", |row| row.get(0))
        .map_err(|_error| "could not inspect SQLite page count".to_string())?;
    let page_size: i64 = connection
        .pragma_query_value(None, "page_size", |row| row.get(0))
        .map_err(|_error| "could not inspect SQLite page size".to_string())?;
    let bytes = page_count
        .checked_mul(page_size)
        .ok_or_else(|| "SQLite page size exceeds the safe preview limit".to_string())?;
    if bytes < 0 || u64::try_from(bytes).unwrap_or(u64::MAX) > MAX_SQLITE_FILE_BYTES {
        return Err("SQLite page size exceeds the safe preview limit".into());
    }
    Ok(())
}

fn table_exists(connection: &Connection, table: &str) -> Result<bool, String> {
    connection
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1 LIMIT 1",
            [table],
            |_| Ok(true),
        )
        .optional()
        .map(|value| value.unwrap_or(false))
        .map_err(|_error| "sqlite_schema_inspection_failed".to_string())
}

fn read_task_run_candidates(
    source: &LegacySqliteSource,
) -> Result<Vec<LegacyImportPreviewItem>, String> {
    let mut connection = open_read_only(source)?;
    let transaction = connection
        .transaction()
        .map_err(|_error| "could not start task-run read transaction".to_string())?;
    validate_sqlite_bounds(&transaction)?;
    for table in [
        "agent_event_journal_task_runs",
        "agent_event_journal_runs",
        "agent_event_journal_events",
    ] {
        if !table_exists(&transaction, table)? {
            return Err("task_run_schema_unsupported".into());
        }
    }
    let task_run_count: i64 = transaction
        .query_row(
            "SELECT COUNT(*) FROM (SELECT 1 FROM agent_event_journal_task_runs LIMIT 5001)",
            [],
            |row| row.get(0),
        )
        .map_err(|_error| "task_run_count_failed".to_string())?;
    if task_run_count > MAX_TASK_RUN_RECORDS {
        return Err(format!(
            "task-run journal exceeds the {MAX_TASK_RUN_RECORDS}-record preview limit"
        ));
    }
    let mut statement = transaction
        .prepare(
            r#"
            SELECT
                   CASE WHEN length(task.chat_id) <= 4096 THEN task.chat_id END,
                   CASE WHEN length(task.title) <= 16384 THEN task.title END,
                   task.created_at_ms,
                   latest.run_id IS NOT NULL,
                   CASE WHEN length(latest.terminal_kind) <= 128
                        THEN latest.terminal_kind END,
                   CASE WHEN length(latest.terminal_payload_json) <= 262144
                        THEN latest.terminal_payload_json END,
                   length(latest.terminal_payload_json),
                   length(latest.terminal_kind)
            FROM agent_event_journal_task_runs AS task
            LEFT JOIN agent_event_journal_runs AS latest
              ON latest.run_id = (
                SELECT events.run_id
                FROM agent_event_journal_events AS events
                WHERE events.chat_id = task.chat_id
                ORDER BY events.recorded_at_ms DESC, events.run_id DESC, events.seq DESC
                LIMIT 1
              )
            ORDER BY task.chat_id ASC
            LIMIT 5001
            "#,
        )
        .map_err(|_error| "task_run_query_prepare_failed".to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, Option<String>>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, bool>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<i64>>(6)?,
                row.get::<_, Option<i64>>(7)?,
            ))
        })
        .map_err(|_error| "task_run_query_failed".to_string())?;

    let mut items = Vec::new();
    let mut decoded_bytes = 0usize;
    for row in rows {
        let (
            chat_id,
            title,
            created_at_ms,
            has_run,
            terminal_kind,
            terminal_payload_json,
            payload_len,
            terminal_kind_len,
        ) = row.map_err(|_error| "task_run_decode_failed".to_string())?;
        let (Some(chat_id), Some(title)) = (chat_id, title) else {
            items.push(LegacyImportPreviewItem::error(
                LegacyImportSourceKind::TaskRun,
                "task-run chat id/title exceeds the preview field limit",
            ));
            continue;
        };
        if chat_id.len() > MAX_ID_BYTES || title.len() > MAX_TITLE_BYTES {
            items.push(LegacyImportPreviewItem::error(
                LegacyImportSourceKind::TaskRun,
                "task-run chat id/title exceeds the preview field limit",
            ));
            continue;
        }
        if terminal_kind.as_ref().is_some_and(|kind| kind.len() > 128) {
            items.push(LegacyImportPreviewItem::error(
                LegacyImportSourceKind::TaskRun,
                "task-run terminal kind exceeds the preview field limit",
            ));
            continue;
        }
        if terminal_payload_json
            .as_ref()
            .is_some_and(|payload| payload.len() > MAX_SQLITE_TEXT_BYTES as usize)
        {
            items.push(LegacyImportPreviewItem::error(
                LegacyImportSourceKind::TaskRun,
                "task-run terminal payload exceeds the preview field limit",
            ));
            continue;
        }
        let row_bytes = chat_id
            .len()
            .saturating_add(title.len())
            .saturating_add(terminal_kind.as_ref().map_or(0, String::len))
            .saturating_add(terminal_payload_json.as_ref().map_or(0, String::len));
        decoded_bytes = decoded_bytes.saturating_add(row_bytes);
        if decoded_bytes > MAX_SQLITE_DECODED_BYTES {
            return Err("task-run journal exceeds the safe decoded-byte limit".into());
        }
        if payload_len.is_some_and(|len| len > MAX_SQLITE_TEXT_BYTES) {
            items.push(LegacyImportPreviewItem::error(
                LegacyImportSourceKind::TaskRun,
                "task-run terminal payload exceeds the preview field limit",
            ));
            continue;
        }
        if terminal_kind_len.is_some_and(|len| len > 128) {
            items.push(LegacyImportPreviewItem::error(
                LegacyImportSourceKind::TaskRun,
                "task-run terminal kind exceeds the preview field limit",
            ));
            continue;
        }
        if chat_id.trim().is_empty() || title.trim().is_empty() || created_at_ms < 0 {
            items.push(LegacyImportPreviewItem::error(
                LegacyImportSourceKind::TaskRun,
                "task-run chat id/title must be non-empty and created_at_ms non-negative",
            ));
            continue;
        }
        let Some(source_key) = checked_source_key(legacy_task_run_source_key(chat_id.trim()))
        else {
            items.push(LegacyImportPreviewItem::error(
                LegacyImportSourceKind::TaskRun,
                "task-run source key exceeds the preview field limit",
            ));
            continue;
        };
        let mapped = match (has_run, terminal_kind.as_deref(), terminal_payload_json) {
            (false, None, None) => LegacyImportPreviewItem::candidate(
                LegacyImportSourceKind::TaskRun,
                source_key,
                title.trim().to_string(),
                "ready",
                "recorded task run has no durable execution and can start a canonical Attempt",
            ),
            (true, None, None) => LegacyImportPreviewItem::candidate(
                LegacyImportSourceKind::TaskRun,
                source_key,
                title.trim().to_string(),
                "in_progress",
                "latest durable task run is still non-terminal",
            ),
            (true, Some("run_terminated"), Some(payload_json)) => {
                map_task_run_terminal(source_key, title.trim(), &payload_json)
            }
            _ => LegacyImportPreviewItem::error(
                LegacyImportSourceKind::TaskRun,
                "task-run record has an inconsistent terminal summary",
            ),
        };
        items.push(mapped);
    }
    drop(statement);
    transaction
        .rollback()
        .map_err(|_error| "could not close task-run read transaction".to_string())?;
    validate_sqlite_source_identity(source)?;
    Ok(items)
}

fn map_task_run_terminal(
    source_key: String,
    title: &str,
    payload_json: &str,
) -> LegacyImportPreviewItem {
    let payload: Value = match serde_json::from_str(payload_json) {
        Ok(payload) => payload,
        Err(_error) => {
            return LegacyImportPreviewItem::error(
                LegacyImportSourceKind::TaskRun,
                "task-run terminal payload is invalid JSON",
            )
        }
    };
    let outcome = payload
        .get("outcome")
        .and_then(|outcome| outcome.get("kind"))
        .and_then(Value::as_str);
    let (state, reason) = match outcome {
        Some("completed") => (
            "in_review",
            "legacy terminal success requires human Accept or Return",
        ),
        Some("failed") => ("ready", "legacy task run failed and can be retried"),
        Some("cancelled") => ("cancelled", "legacy task run was cancelled"),
        Some(_other) => {
            return LegacyImportPreviewItem::error(
                LegacyImportSourceKind::TaskRun,
                "task-run terminal outcome is unsupported",
            )
        }
        None => {
            return LegacyImportPreviewItem::error(
                LegacyImportSourceKind::TaskRun,
                "task-run terminal payload is missing outcome.kind",
            )
        }
    };
    LegacyImportPreviewItem::candidate(
        LegacyImportSourceKind::TaskRun,
        source_key,
        title.to_string(),
        state,
        reason,
    )
}

fn read_existing_source_markers(source: &LegacySqliteSource) -> Result<HashSet<String>, String> {
    let mut connection = open_read_only(source)?;
    let transaction = connection
        .transaction()
        .map_err(|_error| "could not start Work dedupe read transaction".to_string())?;
    validate_sqlite_bounds(&transaction)?;
    if !table_exists(&transaction, "work_events")? {
        return Err("existing work.db is missing required read-only table work_events".to_string());
    }
    let marker_count: i64 = transaction
        .query_row(
            "SELECT COUNT(*) FROM (
                SELECT 1 FROM work_events WHERE kind = ?1 LIMIT 10001
             )",
            [LEGACY_IMPORT_EVENT_KIND],
            |row| row.get(0),
        )
        .map_err(|_error| "work_marker_count_failed".to_string())?;
    if marker_count > MAX_MARKER_RECORDS {
        return Err(format!(
            "Work source markers exceed the {MAX_MARKER_RECORDS}-record preview limit"
        ));
    }
    let mut statement = transaction
        .prepare(
            "SELECT CASE WHEN length(payload_json) <= 4096 THEN payload_json END,
                    length(payload_json)
             FROM work_events WHERE kind = ?1 ORDER BY id ASC LIMIT 10001",
        )
        .map_err(|_error| "work_marker_query_prepare_failed".to_string())?;
    let payloads = statement
        .query_map([LEGACY_IMPORT_EVENT_KIND], |row| {
            Ok((row.get::<_, Option<String>>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|_error| "work_marker_query_failed".to_string())?;
    let mut markers = HashSet::new();
    let mut decoded_bytes = 0usize;
    for payload in payloads {
        let (payload, payload_len) =
            payload.map_err(|_error| "work_marker_decode_failed".to_string())?;
        if payload_len > MAX_MARKER_PAYLOAD_BYTES {
            return Err("legacy import Work event payload exceeds the preview field limit".into());
        }
        let payload = payload.ok_or_else(|| {
            "legacy import Work event payload could not be read within the preview limit"
                .to_string()
        })?;
        if payload.len() > MAX_MARKER_PAYLOAD_BYTES as usize {
            return Err("legacy import Work event payload exceeds the preview field limit".into());
        }
        decoded_bytes = decoded_bytes.saturating_add(payload.len());
        if decoded_bytes > MAX_SQLITE_DECODED_BYTES {
            return Err("Work source markers exceed the safe decoded-byte limit".into());
        }
        let value: Value = serde_json::from_str(&payload)
            .map_err(|_error| "work_marker_json_invalid".to_string())?;
        let marker = value
            .get(LEGACY_IMPORT_SOURCE_KEY_FIELD)
            .and_then(Value::as_str)
            .ok_or_else(|| {
                format!("legacy import Work event is missing {LEGACY_IMPORT_SOURCE_KEY_FIELD}")
            })?;
        if marker.len() > LEGACY_IMPORT_SOURCE_KEY_MAX_BYTES {
            return Err(
                "legacy import Work event source key exceeds the preview field limit".into(),
            );
        }
        markers.insert(marker.to_string());
    }
    drop(statement);
    transaction
        .rollback()
        .map_err(|_error| "could not close Work dedupe read transaction".to_string())?;
    validate_sqlite_source_identity(source)?;
    Ok(markers)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::EventJournal;
    use rusqlite::params;
    use std::hash::{Hash, Hasher};
    use std::sync::{Arc, Barrier};

    const ASSIGNMENTS_FIXTURE: &[u8] =
        include_bytes!("test_fixtures/legacy_import/assignments.json");
    const TODOS_FIXTURE: &[u8] = include_bytes!("test_fixtures/legacy_import/todos.json");

    fn assignments_fixture(workspace: &Path) -> Vec<u8> {
        let mut value: Value = serde_json::from_slice(ASSIGNMENTS_FIXTURE).expect("fixture JSON");
        for assignment in value["assignments"]
            .as_array_mut()
            .expect("assignment array")
        {
            if let Some(path) = assignment
                .get_mut("runConfig")
                .and_then(Value::as_object_mut)
                .and_then(|config| config.get_mut("workspacePath"))
            {
                *path = Value::String(workspace.to_string_lossy().into_owned());
            }
            if let Some(key) = assignment
                .get_mut("orchestration")
                .and_then(Value::as_object_mut)
                .and_then(|orchestration| orchestration.get_mut("workspaceKey"))
            {
                *key = Value::String(workspace.to_string_lossy().into_owned());
            }
        }
        serde_json::to_vec(&value).expect("serialize fixture")
    }

    fn file_fingerprint(path: &Path) -> (u64, u64) {
        let bytes = fs::read(path).expect("fixture bytes");
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        bytes.hash(&mut hasher);
        (bytes.len() as u64, hasher.finish())
    }

    fn directory_entry_names(path: &Path) -> Vec<String> {
        let mut names = fs::read_dir(path)
            .expect("fixture directory")
            .map(|entry| {
                entry
                    .expect("directory entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect::<Vec<_>>();
        names.sort();
        names
    }

    fn preview_paths(root: &Path) -> LegacyImportPreviewPaths {
        LegacyImportPreviewPaths {
            selected_workspace: LegacyWorkspaceRoot::capture(root.to_path_buf())
                .expect("workspace root token"),
            assignments_json: LegacyImportSource::missing(root.join("assignments.json")),
            todos_json: LegacyImportSource::missing(root.join("todos.json")),
            task_run_journal_db: LegacySqliteSource::missing(root.join("journal.db")),
            work_db: LegacySqliteSource::missing(root.join("work.db")),
        }
    }

    fn capture_present_sources(paths: &mut LegacyImportPreviewPaths) {
        for source in [&mut paths.assignments_json, &mut paths.todos_json] {
            if source.path.exists() {
                *source = LegacyImportSource::capture(source.path.clone()).expect("capture source");
            }
        }
        for source in [&mut paths.task_run_journal_db, &mut paths.work_db] {
            if source.main.path.exists() {
                *source = LegacySqliteSource::capture(source.main.path.clone())
                    .expect("capture database");
            }
        }
    }

    fn create_journal_fixture(path: &Path) {
        let connection = Connection::open(path).expect("journal fixture");
        connection
            .execute_batch(
                r#"
                CREATE TABLE agent_event_journal_task_runs (
                    chat_id TEXT PRIMARY KEY, title TEXT NOT NULL, created_at_ms INTEGER NOT NULL
                );
                CREATE TABLE agent_event_journal_runs (
                    run_id TEXT PRIMARY KEY, chat_id TEXT NOT NULL, last_seq INTEGER NOT NULL,
                    terminal_seq INTEGER, terminal_kind TEXT, terminal_payload_json TEXT
                );
                CREATE TABLE agent_event_journal_events (
                    run_id TEXT NOT NULL, seq INTEGER NOT NULL, chat_id TEXT NOT NULL,
                    recorded_at_ms INTEGER NOT NULL
                );
                CREATE INDEX agent_event_journal_events_chat_time
                    ON agent_event_journal_events(chat_id, recorded_at_ms, run_id, seq);
                INSERT INTO agent_event_journal_task_runs VALUES
                    ('chat-active', 'Active task', 10),
                    ('chat-done', 'Finished task', 20);
                INSERT INTO agent_event_journal_runs VALUES
                    ('run-active', 'chat-active', 1, NULL, NULL, NULL),
                    ('run-done', 'chat-done', 2, 2, 'run_terminated',
                     '{"outcome":{"kind":"completed"}}');
                INSERT INTO agent_event_journal_events VALUES
                    ('run-active', 1, 'chat-active', 11),
                    ('run-done', 2, 'chat-done', 22);
                "#,
            )
            .expect("journal schema");
    }

    fn create_work_fixture(path: &Path, marker: &str) {
        let connection = Connection::open(path).expect("work fixture");
        connection
            .execute_batch(
                "CREATE TABLE work_events (id INTEGER PRIMARY KEY, kind TEXT NOT NULL, payload_json TEXT NOT NULL);",
            )
            .expect("work schema");
        connection
            .execute(
                "INSERT INTO work_events (kind, payload_json) VALUES (?1, ?2)",
                params![
                    LEGACY_IMPORT_EVENT_KIND,
                    serde_json::json!({LEGACY_IMPORT_SOURCE_KEY_FIELD: marker}).to_string()
                ],
            )
            .expect("marker");
    }

    #[test]
    fn assignment_fixture_never_maps_terminal_success_to_done() {
        let root = tempfile::tempdir().expect("tempdir");
        let fixture = assignments_fixture(root.path());
        let items = parse_legacy_assignments(&fixture, root.path());
        assert_eq!(items.len(), 3);
        assert_eq!(
            items
                .iter()
                .find(|item| {
                    item.source_key.as_deref() == Some("legacy:v1:assignment:8:asg-done")
                })
                .and_then(|item| item.proposed_work_state.as_deref()),
            Some("in_review")
        );
        assert!(items
            .iter()
            .all(|item| item.proposed_work_state.as_deref() != Some("done")));
    }

    #[test]
    fn global_todo_fixture_is_skipped_without_workspace_provenance() {
        let root = tempfile::tempdir().expect("tempdir");
        let items = parse_legacy_todos(TODOS_FIXTURE, root.path());
        assert_eq!(
            count_disposition(&items, LegacyImportDisposition::Candidate),
            0
        );
        assert_eq!(
            count_disposition(&items, LegacyImportDisposition::Skipped),
            4
        );
        assert!(items.iter().any(|item| {
            item.source_key.as_deref() == Some("legacy:v1:todo:9:session-a:15:manual-complete")
                && item.proposed_work_state.is_none()
                && item.reason == "manual todo has no proven provenance for the selected workspace"
        }));
    }

    #[test]
    fn preview_is_byte_for_byte_read_only_and_dedupes_exact_marker() {
        let root = tempfile::tempdir().expect("tempdir");
        let mut paths = preview_paths(root.path());
        fs::write(&paths.assignments_json, assignments_fixture(root.path()))
            .expect("assignments fixture");
        fs::write(&paths.todos_json, TODOS_FIXTURE).expect("todos fixture");
        create_journal_fixture(&paths.task_run_journal_db);
        create_work_fixture(&paths.work_db, "legacy:v1:assignment:8:asg-done");
        capture_present_sources(&mut paths);

        let files: [&Path; 4] = [
            paths.assignments_json.as_ref(),
            paths.todos_json.as_ref(),
            paths.task_run_journal_db.as_ref(),
            paths.work_db.as_ref(),
        ];
        let before = files.map(|path| file_fingerprint(path));
        let entries_before = directory_entry_names(root.path());
        let report = preview_legacy_import(&paths);
        let after = files.map(|path| file_fingerprint(path));
        let entries_after = directory_entry_names(root.path());

        assert_eq!(before, after);
        assert_eq!(entries_before, entries_after);
        assert_eq!(report.candidate_count, 4, "{report:#?}");
        assert_eq!(report.skipped_count, 4);
        assert_eq!(report.duplicate_count, 1);
        assert_eq!(report.error_count, 0);
        assert!(report.items.iter().any(|item| {
            item.source_key.as_deref() == Some("legacy:v1:task-run:9:chat-done")
                && item.proposed_work_state.as_deref() == Some("in_review")
        }));
    }

    #[test]
    fn missing_work_db_is_tolerated_and_not_created() {
        let root = tempfile::tempdir().expect("tempdir");
        let mut paths = preview_paths(root.path());
        fs::write(&paths.assignments_json, assignments_fixture(root.path()))
            .expect("assignments fixture");
        capture_present_sources(&mut paths);

        assert!(!paths.work_db.exists());
        let entries_before = directory_entry_names(root.path());
        let report = preview_legacy_import(&paths);
        assert!(!paths.work_db.exists());
        assert_eq!(entries_before, directory_entry_names(root.path()));
        assert_eq!(report.error_count, 0);
        assert_eq!(report.candidate_count, 3);
    }

    #[test]
    fn malformed_records_are_errors_without_guessing() {
        let root = tempfile::tempdir().expect("tempdir");
        let assignments = parse_legacy_assignments(br#"{"assignments":[{"id":1}]}"#, root.path());
        let todos = parse_legacy_todos(
            br#"{"todos:s":[{"origin":"manual","status":"mystery"}]}"#,
            root.path(),
        );
        assert_eq!(
            count_disposition(&assignments, LegacyImportDisposition::Error),
            1
        );
        assert_eq!(count_disposition(&todos, LegacyImportDisposition::Error), 1);
    }

    #[test]
    fn missing_source_is_empty_but_present_invalid_source_is_an_error() {
        let root = tempfile::tempdir().expect("tempdir");
        let mut paths = preview_paths(root.path());
        let missing = preview_legacy_import(&paths);
        assert_eq!(missing.error_count, 0);

        fs::write(&paths.assignments_json, b"not json").expect("invalid source");
        capture_present_sources(&mut paths);
        let invalid = preview_legacy_import(&paths);
        assert_eq!(invalid.error_count, 1);
    }

    #[test]
    fn oversized_json_is_rejected_before_parsing() {
        let root = tempfile::tempdir().expect("tempdir");
        let mut paths = preview_paths(root.path());
        let oversized = vec![b' '; MAX_LEGACY_JSON_BYTES as usize + 1];
        fs::write(&paths.assignments_json, oversized).expect("oversized source");
        capture_present_sources(&mut paths);
        let report = preview_legacy_import(&paths);
        assert_eq!(report.error_count, 1);
        assert!(report.items[0].reason.contains("preview limit"));
    }

    #[cfg(unix)]
    #[test]
    fn symlink_sources_are_rejected_without_following_them() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().expect("tempdir");
        let paths = preview_paths(root.path());
        let target = root.path().join("target.json");
        fs::write(&target, ASSIGNMENTS_FIXTURE).expect("target");
        symlink(&target, &paths.assignments_json).expect("symlink");

        assert!(LegacyImportSource::capture(paths.assignments_json.path.clone()).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn source_open_itself_does_not_follow_a_swapped_symlink() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().expect("tempdir");
        let target = root.path().join("target.json");
        let source = root.path().join("source.json");
        fs::write(&target, b"{}").expect("target");
        symlink(&target, &source).expect("symlink");

        assert!(open_source_file_no_follow(&source).is_err());
    }

    #[test]
    fn report_order_is_deterministic() {
        let root = tempfile::tempdir().expect("tempdir");
        let mut paths = preview_paths(root.path());
        fs::write(&paths.assignments_json, assignments_fixture(root.path()))
            .expect("assignments fixture");
        fs::write(&paths.todos_json, TODOS_FIXTURE).expect("todos fixture");
        capture_present_sources(&mut paths);
        let first = preview_legacy_import(&paths);
        let second = preview_legacy_import(&paths);
        assert_eq!(first, second);
    }

    #[test]
    fn duplicate_legacy_keys_are_counted_without_title_heuristics() {
        let root = tempfile::tempdir().expect("tempdir");
        let mut paths = preview_paths(root.path());
        let fixture = serde_json::json!({"assignments": [
          {"id":"same","source":{"kind":"task","prompt":"a"},"sessionId":"s1","title":"A","status":"running","runConfig":{"workspacePath":root.path()},"createdAt":1,"updatedAt":1},
          {"id":"same","source":{"kind":"task","prompt":"b"},"sessionId":"s2","title":"B","status":"failed","runConfig":{"workspacePath":root.path()},"createdAt":2,"updatedAt":2}
        ]});
        fs::write(
            &paths.assignments_json,
            serde_json::to_vec(&fixture).expect("serialize assignments"),
        )
        .expect("assignments");
        capture_present_sources(&mut paths);
        let report = preview_legacy_import(&paths);
        assert_eq!(report.candidate_count, 1);
        assert_eq!(report.duplicate_count, 1);
    }

    #[test]
    fn assignment_provenance_is_exact_to_the_selected_workspace() {
        let workspace_a = tempfile::tempdir().expect("workspace a");
        let workspace_b = tempfile::tempdir().expect("workspace b");
        let fixture = assignments_fixture(workspace_a.path());

        let for_a = parse_legacy_assignments(&fixture, workspace_a.path());
        let for_b = parse_legacy_assignments(&fixture, workspace_b.path());

        assert_eq!(
            count_disposition(&for_a, LegacyImportDisposition::Candidate),
            3
        );
        assert_eq!(
            count_disposition(&for_b, LegacyImportDisposition::Candidate),
            0
        );
        assert_eq!(
            count_disposition(&for_b, LegacyImportDisposition::Skipped),
            3
        );
        assert!(for_b
            .iter()
            .all(|item| item.reason
                == "assignment has no proven provenance for the selected workspace"));
    }

    #[test]
    fn conflicting_workspace_anchors_are_skipped() {
        let selected = tempfile::tempdir().expect("selected");
        let other = tempfile::tempdir().expect("other");
        let fixture = serde_json::json!({"assignments": [{
            "id":"a", "source":{"kind":"task","prompt":"p"}, "sessionId":"s",
            "title":"A", "status":"running", "origin":"orchestrator",
            "runConfig":{"workspacePath":selected.path()},
            "orchestration":{"workspaceKey":other.path(),"taskSessionId":"ts","taskKey":"tk","attempt":1},
            "createdAt":1, "updatedAt":1
        }]});
        let items = parse_legacy_assignments(
            &serde_json::to_vec(&fixture).expect("fixture"),
            selected.path(),
        );
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].disposition, LegacyImportDisposition::Skipped);
        assert_eq!(
            items[0].reason,
            "assignment has no proven provenance for the selected workspace"
        );
    }

    #[test]
    fn todo_orchestration_identifiers_are_bounded_before_source_key_creation() {
        let selected = tempfile::tempdir().expect("selected");
        let oversized = "x".repeat(MAX_ID_BYTES + 1);
        let fixture = serde_json::json!({"assignments": [{
            "id":"a", "source":{"kind":"todo","todoId":oversized.clone()}, "sessionId":"s",
            "title":"A", "status":"running", "origin":"orchestrator",
            "orchestration":{"workspaceKey":selected.path(),"taskSessionId":oversized.clone(),"taskKey":oversized,"attempt":1},
            "createdAt":1, "updatedAt":1
        }]});
        let items = parse_legacy_assignments(
            &serde_json::to_vec(&fixture).expect("fixture"),
            selected.path(),
        );
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].disposition, LegacyImportDisposition::Skipped);
        assert_eq!(
            items[0].reason,
            "assignment todo provenance is inconsistent"
        );
        assert!(items[0]
            .source_key
            .as_ref()
            .is_some_and(|key| key.len() < MAX_ID_BYTES));
    }

    #[test]
    fn unscoped_known_records_are_skipped_before_status_interpretation() {
        let workspace = tempfile::tempdir().expect("workspace");
        let assignment = br#"{"assignments":[{
            "id":"a","source":{"kind":"task","prompt":"p"},"sessionId":"s",
            "title":"A","status":"private-status","origin":"manual",
            "createdAt":1,"updatedAt":1
        }]}"#;
        let todo = br#"{"todos:s":[{
            "id":"t","title":"T","status":"private-status","origin":"manual"
        }]}"#;

        let assignments = parse_legacy_assignments(assignment, workspace.path());
        let todos = parse_legacy_todos(todo, workspace.path());
        assert_eq!(assignments[0].disposition, LegacyImportDisposition::Skipped);
        assert_eq!(todos[0].disposition, LegacyImportDisposition::Skipped);
        assert!(!assignments[0].reason.contains("private-status"));
        assert!(!todos[0].reason.contains("private-status"));
    }

    #[test]
    fn orchestrated_todo_assignment_has_one_logical_candidate() {
        let workspace = tempfile::tempdir().expect("workspace");
        let assignments_fixture = assignments_fixture(workspace.path());
        let todo_fixture = br#"{"todos:session-a":[
            {"id":"todo-7","title":"Orchestrated todo","status":"pending","origin":"manual"}
        ]}"#;
        let mut items = parse_legacy_assignments(&assignments_fixture, workspace.path());
        items.extend(parse_legacy_todos(todo_fixture, workspace.path()));
        classify_duplicates(&mut items, Some(&HashSet::new()));

        let logical_key = legacy_todo_source_key("session-a", "todo-7");
        let matching = items
            .iter()
            .filter(|item| item.source_key.as_deref() == Some(logical_key.as_str()))
            .collect::<Vec<_>>();
        assert_eq!(matching.len(), 2);
        assert_eq!(
            matching
                .iter()
                .filter(|item| item.disposition == LegacyImportDisposition::Candidate)
                .count(),
            1
        );
        assert_eq!(
            matching
                .iter()
                .filter(|item| item.disposition == LegacyImportDisposition::Skipped)
                .count(),
            1
        );
    }

    #[test]
    fn json_record_overflow_is_one_fixed_error_without_amplification() {
        let workspace = tempfile::tempdir().expect("workspace");
        let assignments = (0..=MAX_JSON_SOURCE_ITEMS)
            .map(|index| serde_json::json!({"id": index}))
            .collect::<Vec<_>>();
        let bytes = serde_json::to_vec(&serde_json::json!({"assignments": assignments}))
            .expect("overflow fixture");

        let items = parse_legacy_assignments(&bytes, workspace.path());
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].disposition, LegacyImportDisposition::Error);
        assert_eq!(
            items[0].reason,
            "assignment source exceeds the safe record limit"
        );
    }

    #[test]
    fn unsupported_values_are_not_reflected_in_errors() {
        let workspace = tempfile::tempdir().expect("workspace");
        let secret = "private-sensitive-value";
        let fixture = serde_json::json!({"assignments": [{
            "id":"a", "source":{"kind":"task","prompt":"p"}, "sessionId":"s",
            "title":"A", "status":secret, "origin":"manual",
            "runConfig":{"workspacePath":workspace.path()}, "createdAt":1, "updatedAt":1
        }]});
        let items = parse_legacy_assignments(
            &serde_json::to_vec(&fixture).expect("fixture"),
            workspace.path(),
        );
        assert_eq!(items.len(), 1);
        assert!(!items[0].reason.contains(secret));
    }

    #[test]
    fn wal_mode_main_only_work_db_uses_an_unchanged_snapshot_source() {
        let root = tempfile::tempdir().expect("tempdir");
        let mut paths = preview_paths(root.path());
        fs::write(&paths.assignments_json, assignments_fixture(root.path()))
            .expect("assignments fixture");
        {
            let connection = Connection::open(&paths.work_db).expect("work fixture");
            connection
                .pragma_update(None, "journal_mode", "WAL")
                .expect("WAL mode");
            connection
                .execute_batch(
                    "CREATE TABLE work_events (
                        id INTEGER PRIMARY KEY, kind TEXT NOT NULL, payload_json TEXT NOT NULL
                     );",
                )
                .expect("work schema");
            connection
                .execute(
                    "INSERT INTO work_events (kind, payload_json) VALUES (?1, ?2)",
                    params![
                        LEGACY_IMPORT_EVENT_KIND,
                        serde_json::json!({
                            LEGACY_IMPORT_SOURCE_KEY_FIELD: "legacy:v1:assignment:8:asg-done"
                        })
                        .to_string()
                    ],
                )
                .expect("marker");
            connection
                .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))
                .expect("checkpoint");
        }
        for suffix in ["-wal", "-shm"] {
            let sidecar = PathBuf::from(format!("{}{}", paths.work_db.display(), suffix));
            if sidecar.exists() {
                fs::remove_file(sidecar).expect("remove closed fixture sidecar");
            }
        }
        let header = fs::read(&paths.work_db).expect("database header");
        assert!(header.len() >= 20);
        assert_eq!(
            (header[18], header[19]),
            (2, 2),
            "fixture must remain WAL mode"
        );
        capture_present_sources(&mut paths);

        let fingerprint_before = file_fingerprint(&paths.work_db);
        let entries_before = directory_entry_names(root.path());
        let report = preview_legacy_import(&paths);

        assert_eq!(file_fingerprint(&paths.work_db), fingerprint_before);
        assert_eq!(directory_entry_names(root.path()), entries_before);
        assert_eq!(report.duplicate_count, 1, "{report:#?}");
        assert_eq!(report.error_count, 0, "{report:#?}");
    }

    #[test]
    fn live_wal_pair_is_read_without_mutating_source_files() {
        let root = tempfile::tempdir().expect("tempdir");
        let mut paths = preview_paths(root.path());
        fs::write(&paths.assignments_json, assignments_fixture(root.path()))
            .expect("assignments fixture");
        let writer = Connection::open(&paths.work_db).expect("work writer");
        writer
            .execute_batch(
                "PRAGMA journal_mode=WAL;
                 PRAGMA wal_autocheckpoint=0;
                 CREATE TABLE work_events (
                    id INTEGER PRIMARY KEY, kind TEXT NOT NULL, payload_json TEXT NOT NULL
                 );",
            )
            .expect("live WAL schema");
        writer
            .execute(
                "INSERT INTO work_events (kind, payload_json) VALUES (?1, ?2)",
                params![
                    LEGACY_IMPORT_EVENT_KIND,
                    serde_json::json!({
                        LEGACY_IMPORT_SOURCE_KEY_FIELD: "legacy:v1:assignment:8:asg-done"
                    })
                    .to_string()
                ],
            )
            .expect("uncheckpointed marker");
        capture_present_sources(&mut paths);
        assert!(paths.work_db.wal.is_some());
        assert!(paths.work_db.shm.is_some());

        let wal_path = sqlite_sidecar_path(&paths.work_db.main.path, "-wal");
        let shm_path = sqlite_sidecar_path(&paths.work_db.main.path, "-shm");
        let before = [
            file_fingerprint(&paths.work_db),
            file_fingerprint(&wal_path),
            file_fingerprint(&shm_path),
        ];
        let entries_before = directory_entry_names(root.path());
        let report = preview_legacy_import(&paths);

        assert_eq!(report.duplicate_count, 1, "{report:#?}");
        assert_eq!(report.error_count, 0, "{report:#?}");
        assert_eq!(
            [
                file_fingerprint(&paths.work_db),
                file_fingerprint(&wal_path),
                file_fingerprint(&shm_path),
            ],
            before
        );
        assert_eq!(directory_entry_names(root.path()), entries_before);
        drop(writer);
    }

    #[test]
    fn concurrent_writer_checkpoint_never_admits_a_mixed_wal_generation() {
        let root = tempfile::tempdir().expect("tempdir");
        let mut paths = preview_paths(root.path());
        fs::write(&paths.assignments_json, assignments_fixture(root.path()))
            .expect("assignments fixture");
        let writer = Connection::open(&paths.work_db).expect("work writer");
        writer
            .execute_batch(
                "PRAGMA journal_mode=WAL;
                 PRAGMA wal_autocheckpoint=0;
                 CREATE TABLE work_events (
                    id INTEGER PRIMARY KEY, kind TEXT NOT NULL, payload_json TEXT NOT NULL
                 );",
            )
            .expect("live WAL schema");
        writer
            .execute(
                "INSERT INTO work_events (kind, payload_json) VALUES (?1, ?2)",
                params![
                    LEGACY_IMPORT_EVENT_KIND,
                    serde_json::json!({
                        LEGACY_IMPORT_SOURCE_KEY_FIELD: "legacy:v1:assignment:8:asg-done"
                    })
                    .to_string()
                ],
            )
            .expect("durable marker");
        capture_present_sources(&mut paths);

        let start = Arc::new(Barrier::new(2));
        let writer_start = Arc::clone(&start);
        let writer_thread = std::thread::spawn(move || {
            writer_start.wait();
            for index in 0..64 {
                writer
                    .execute(
                        "INSERT INTO work_events (kind, payload_json) VALUES ('noise', ?1)",
                        [format!("{{\"index\":{index}}}")],
                    )
                    .expect("concurrent write");
                writer
                    .execute_batch("PRAGMA wal_checkpoint(PASSIVE);")
                    .expect("concurrent checkpoint");
            }
            writer
        });
        start.wait();
        let concurrent_report = preview_legacy_import(&paths);
        let writer = writer_thread.join().expect("writer thread");

        let coherent = concurrent_report.candidate_count == 2
            && concurrent_report.duplicate_count == 1
            && concurrent_report.error_count == 0;
        let failed_closed = concurrent_report.candidate_count == 0
            && concurrent_report.duplicate_count == 0
            && concurrent_report.error_count > 0;
        assert!(coherent || failed_closed, "{concurrent_report:#?}");

        // Once the writer is quiescent, the preview must see the marker and
        // must not alter any of the live source files or directory entries.
        capture_present_sources(&mut paths);
        let wal_path = sqlite_sidecar_path(&paths.work_db.main.path, "-wal");
        let shm_path = sqlite_sidecar_path(&paths.work_db.main.path, "-shm");
        let before = [
            file_fingerprint(&paths.work_db),
            file_fingerprint(&wal_path),
            file_fingerprint(&shm_path),
        ];
        let entries_before = directory_entry_names(root.path());
        let stable_report = preview_legacy_import(&paths);
        assert_eq!(stable_report.candidate_count, 2, "{stable_report:#?}");
        assert_eq!(stable_report.duplicate_count, 1, "{stable_report:#?}");
        assert_eq!(stable_report.error_count, 0, "{stable_report:#?}");
        assert_eq!(
            [
                file_fingerprint(&paths.work_db),
                file_fingerprint(&wal_path),
                file_fingerprint(&shm_path),
            ],
            before
        );
        assert_eq!(directory_entry_names(root.path()), entries_before);
        drop(writer);
    }

    #[test]
    fn active_delete_mode_rollback_journal_is_rejected_without_opening_source() {
        let root = tempfile::tempdir().expect("tempdir");
        let db = root.path().join("journal.db");
        let journal = EventJournal::open(&db).expect("event journal");
        journal
            .create_task_run("chat-base", "Baseline task")
            .expect("baseline task");
        let captured = LegacySqliteSource::capture(db.clone()).expect("captured journal");
        let writer = Connection::open(&db).expect("writer");
        assert_eq!(
            writer
                .pragma_query_value(None, "journal_mode", |row| row.get::<_, String>(0))
                .expect("journal mode"),
            "delete"
        );
        writer
            .execute_batch(
                "BEGIN IMMEDIATE;
                 UPDATE agent_event_journal_task_runs
                 SET title = 'uncommitted' WHERE chat_id = 'chat-base';",
            )
            .expect("uncommitted update");
        let rollback = sqlite_sidecar_path(&db, "-journal");
        assert!(rollback.exists(), "real rollback journal must exist");

        assert_eq!(
            open_read_only(&captured)
                .err()
                .expect("active rollback rejected"),
            "sqlite_rollback_journal_rejected"
        );
        writer.execute_batch("ROLLBACK;").expect("rollback");
        assert!(!rollback.exists());
    }

    #[test]
    fn concurrent_event_journal_append_is_coherent_or_fails_closed_and_never_mutates_source() {
        let root = tempfile::tempdir().expect("tempdir");
        let mut paths = preview_paths(root.path());
        let journal = Arc::new(
            EventJournal::open(&paths.task_run_journal_db.main.path).expect("event journal"),
        );
        journal
            .create_task_run("chat-base", "Baseline task")
            .expect("baseline task");
        capture_present_sources(&mut paths);

        let start = Arc::new(Barrier::new(2));
        let writer_start = Arc::clone(&start);
        let writer_journal = Arc::clone(&journal);
        let writer = std::thread::spawn(move || {
            writer_start.wait();
            for index in 0..64 {
                writer_journal
                    .create_task_run(&format!("chat-{index:02}"), &format!("Task {index:02}"))
                    .expect("concurrent task append");
            }
        });
        start.wait();
        let concurrent_report = preview_legacy_import(&paths);
        writer.join().expect("writer thread");
        let coherent = concurrent_report.error_count == 0
            && concurrent_report.skipped_count == 0
            && concurrent_report.duplicate_count == 0
            && (1..=65).contains(&concurrent_report.candidate_count);
        let failed_closed = concurrent_report.candidate_count == 0
            && concurrent_report.duplicate_count == 0
            && concurrent_report.error_count > 0;
        assert!(coherent || failed_closed, "{concurrent_report:#?}");

        capture_present_sources(&mut paths);
        let db = &paths.task_run_journal_db.main.path;
        let before = file_fingerprint(db);
        let entries_before = directory_entry_names(root.path());
        let stable_report = preview_legacy_import(&paths);
        assert_eq!(stable_report.candidate_count, 65, "{stable_report:#?}");
        assert_eq!(stable_report.error_count, 0, "{stable_report:#?}");
        assert_eq!(file_fingerprint(db), before);
        assert_eq!(directory_entry_names(root.path()), entries_before);
    }

    #[test]
    fn sqlite_identity_token_rejects_parent_path_substitution() {
        let parent = tempfile::tempdir().expect("parent");
        let active = parent.path().join("workspace");
        let moved = parent.path().join("workspace-original");
        fs::create_dir(&active).expect("active");
        let db = active.join("work.db");
        create_work_fixture(&db, "marker");
        let captured = LegacySqliteSource::capture(db.clone()).expect("capture database");

        fs::rename(&active, &moved).expect("move parent");
        fs::create_dir(&active).expect("replacement parent");
        create_work_fixture(&db, "outside-marker");

        assert_eq!(
            open_read_only(&captured).err().expect("identity mismatch"),
            "sqlite_source_identity_changed"
        );
    }

    #[test]
    fn sqlite_identity_token_rejects_a_replaced_wal_sidecar() {
        let root = tempfile::tempdir().expect("root");
        let db = root.path().join("work.db");
        create_work_fixture(&db, "marker");
        let wal = sqlite_sidecar_path(&db, "-wal");
        let shm = sqlite_sidecar_path(&db, "-shm");
        fs::write(&wal, b"captured-wal").expect("wal");
        fs::write(&shm, b"captured-shm").expect("shm");
        let captured = LegacySqliteSource::capture(db).expect("capture database");
        fs::remove_file(&wal).expect("remove wal");
        fs::write(&wal, b"replacement-wal").expect("replacement wal");

        assert_eq!(
            open_read_only(&captured).err().expect("identity mismatch"),
            "sqlite_wal_identity_changed"
        );
    }

    #[test]
    fn report_errors_do_not_echo_paths_or_backend_details() {
        let root = tempfile::tempdir().expect("tempdir");
        let sentinel = "SECRET_BACKEND_PATH_SENTINEL";
        let mut paths = preview_paths(root.path());
        paths.work_db.main.path = root.path().join(format!("{sentinel}.db"));
        fs::write(&paths.assignments_json, assignments_fixture(root.path())).expect("assignments");
        fs::write(&paths.work_db, format!("not sqlite {sentinel}")).expect("invalid database");
        capture_present_sources(&mut paths);

        let report = preview_legacy_import(&paths);
        assert!(report.error_count > 0);
        assert!(report
            .items
            .iter()
            .all(|item| !item.reason.contains(sentinel)));
        assert!(report.items.iter().all(|item| !item
            .reason
            .contains(&root.path().to_string_lossy().into_owned())));
    }

    #[test]
    fn framed_source_keys_cannot_collide_across_component_boundaries() {
        assert_ne!(
            legacy_todo_source_key("a:b", "c"),
            legacy_todo_source_key("a", "b:c")
        );
        assert_ne!(
            legacy_assignment_source_key("same"),
            legacy_task_run_source_key("same")
        );
    }

    #[test]
    fn whole_source_key_byte_cap_accepts_boundary_and_rejects_next_byte() {
        let workspace = tempfile::tempdir().expect("workspace");
        let max_id_len = (1..=LEGACY_IMPORT_SOURCE_KEY_MAX_BYTES)
            .filter(|len| {
                legacy_assignment_source_key(&"a".repeat(*len)).len()
                    <= LEGACY_IMPORT_SOURCE_KEY_MAX_BYTES
            })
            .max()
            .expect("bounded assignment id");
        let boundary_id = "a".repeat(max_id_len);
        let boundary_key = legacy_assignment_source_key(&boundary_id);
        assert_eq!(boundary_key.len(), LEGACY_IMPORT_SOURCE_KEY_MAX_BYTES);
        let escaped_boundary = legacy_assignment_source_key(&"\0".repeat(max_id_len));
        assert_eq!(escaped_boundary.len(), LEGACY_IMPORT_SOURCE_KEY_MAX_BYTES);
        assert!(
            serde_json::json!({(LEGACY_IMPORT_SOURCE_KEY_FIELD): escaped_boundary})
                .to_string()
                .len()
                <= MAX_MARKER_PAYLOAD_BYTES as usize
        );
        let accepted = serde_json::json!({"assignments":[{
            "id":boundary_id, "source":{"kind":"task","prompt":"p"}, "sessionId":"s",
            "title":"Boundary", "status":"running", "origin":"manual",
            "runConfig":{"workspacePath":workspace.path()}, "createdAt":1, "updatedAt":1
        }]});
        let accepted = parse_legacy_assignments(
            &serde_json::to_vec(&accepted).expect("boundary fixture"),
            workspace.path(),
        );
        assert_eq!(accepted[0].disposition, LegacyImportDisposition::Candidate);
        assert_eq!(
            accepted[0].source_key.as_deref(),
            Some(boundary_key.as_str())
        );

        let rejected_id = "a".repeat(max_id_len + 1);
        let rejected = serde_json::json!({"assignments":[{
            "id":rejected_id, "source":{"kind":"task","prompt":"p"}, "sessionId":"s",
            "title":"Too long", "status":"running", "origin":"manual",
            "runConfig":{"workspacePath":workspace.path()}, "createdAt":1, "updatedAt":1
        }]});
        let rejected = parse_legacy_assignments(
            &serde_json::to_vec(&rejected).expect("oversized fixture"),
            workspace.path(),
        );
        assert_eq!(rejected[0].disposition, LegacyImportDisposition::Error);
        assert!(rejected[0].source_key.is_none());
    }

    #[test]
    fn multibyte_and_combined_todo_source_keys_use_utf8_byte_cap() {
        let workspace = tempfile::tempdir().expect("workspace");
        let multibyte_id = "é".repeat(LEGACY_IMPORT_SOURCE_KEY_MAX_BYTES / 2);
        let assignment = serde_json::json!({"assignments":[{
            "id":multibyte_id, "source":{"kind":"task","prompt":"p"}, "sessionId":"s",
            "title":"Multibyte", "status":"running", "origin":"manual",
            "runConfig":{"workspacePath":workspace.path()}, "createdAt":1, "updatedAt":1
        }]});
        let assignment = parse_legacy_assignments(
            &serde_json::to_vec(&assignment).expect("multibyte fixture"),
            workspace.path(),
        );
        assert_eq!(assignment[0].disposition, LegacyImportDisposition::Error);
        assert!(assignment[0].source_key.is_none());

        let session_id = "s".repeat(250);
        let todo_id = "t".repeat(250);
        let todos = serde_json::json!({(format!("todos:{session_id}")):[{
            "id":todo_id, "title":"Combined key", "status":"pending", "origin":"manual"
        }]});
        let todos = parse_legacy_todos(
            &serde_json::to_vec(&todos).expect("combined todo fixture"),
            workspace.path(),
        );
        assert_eq!(todos[0].disposition, LegacyImportDisposition::Error);
        assert!(todos[0].source_key.is_none());
    }

    #[test]
    fn sqlite_ids_and_markers_are_checked_by_decoded_utf8_bytes() {
        let root = tempfile::tempdir().expect("tempdir");
        let mut paths = preview_paths(root.path());
        create_journal_fixture(&paths.task_run_journal_db.main.path);
        let multibyte_chat = "é".repeat(3_000);
        Connection::open(&paths.task_run_journal_db.main.path)
            .expect("journal")
            .execute(
                "INSERT INTO agent_event_journal_task_runs (chat_id, title, created_at_ms)
                 VALUES (?1, 'Multibyte id', 2)",
                [&multibyte_chat],
            )
            .expect("multibyte task row");
        let oversized_marker = "m".repeat(LEGACY_IMPORT_SOURCE_KEY_MAX_BYTES + 1);
        create_work_fixture(&paths.work_db.main.path, &oversized_marker);
        capture_present_sources(&mut paths);

        let report = preview_legacy_import(&paths);
        assert!(report.items.iter().any(|item| {
            item.disposition == LegacyImportDisposition::Error
                && item.reason == "task-run chat id/title exceeds the preview field limit"
        }));
        assert!(report.items.iter().any(|item| {
            item.disposition == LegacyImportDisposition::Error
                && item.reason
                    == "legacy import Work event source key exceeds the preview field limit"
        }));
        assert!(report.items.iter().all(|item| item
            .source_key
            .as_ref()
            .is_none_or(|key| key.len() <= LEGACY_IMPORT_SOURCE_KEY_MAX_BYTES)));
    }

    #[test]
    fn existing_unreadable_work_schema_blocks_unverified_candidates() {
        let root = tempfile::tempdir().expect("tempdir");
        let mut paths = preview_paths(root.path());
        fs::write(&paths.assignments_json, assignments_fixture(root.path()))
            .expect("assignments fixture");
        Connection::open(&paths.work_db).expect("empty work database");
        capture_present_sources(&mut paths);

        let report = preview_legacy_import(&paths);
        assert_eq!(report.candidate_count, 0);
        assert_eq!(report.error_count, 4);
        assert!(!report
            .items
            .iter()
            .any(|item| { item.disposition == LegacyImportDisposition::Candidate }));
    }

    #[test]
    fn parser_output_serializes_as_camel_case_contract() {
        let root = tempfile::tempdir().expect("tempdir");
        let fixture = assignments_fixture(root.path());
        let report =
            LegacyImportPreviewReport::from_items(parse_legacy_assignments(&fixture, root.path()));
        let json = serde_json::to_value(report).expect("serialize report");
        assert!(json.get("candidateCount").is_some());
        assert!(json["items"][0].get("sourceKind").is_some());
        assert!(json["items"][0].get("proposedWorkState").is_some());
    }
}
