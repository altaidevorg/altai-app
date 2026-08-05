//! Native-host provider credentials.
//!
//! Credentials are intentionally owned by the Rust stdio host.  They never
//! appear in protocol responses and callers can only observe whether a
//! provider is configured.  The Webview therefore cannot retrieve a stored
//! credential even when it asks the Extension Host to initiate a connection.

use std::collections::HashMap;
use std::path::PathBuf;

const CREDENTIALS_FILE: &str = "agent-host-credentials.json";

fn credentials_path() -> Result<PathBuf, String> {
    if let Some(directory) = std::env::var_os("ALTAI_CLI_CREDENTIALS_DIR") {
        return Ok(PathBuf::from(directory).join(CREDENTIALS_FILE));
    }
    dirs::data_local_dir()
        .map(|directory| directory.join("ALTAI").join(CREDENTIALS_FILE))
        .ok_or_else(|| "credential_store_unavailable".to_string())
}

fn read_store() -> Result<HashMap<String, String>, String> {
    let path = credentials_path()?;
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let contents = std::fs::read(&path).map_err(|_| "credential_store_unavailable".to_string())?;
    serde_json::from_slice(&contents).map_err(|_| "credential_store_unavailable".to_string())
}

fn write_store(credentials: &HashMap<String, String>) -> Result<(), String> {
    let path = credentials_path()?;
    let parent = path
        .parent()
        .ok_or_else(|| "credential_store_unavailable".to_string())?;
    std::fs::create_dir_all(parent).map_err(|_| "credential_store_unavailable".to_string())?;
    let temporary = path.with_extension("json.tmp");
    let serialized = serde_json::to_vec(credentials)
        .map_err(|_| "credential_store_unavailable".to_string())?;

    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;

        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .mode(0o600)
            .open(&temporary)
            .map_err(|_| "credential_store_unavailable".to_string())?;
        file.write_all(&serialized)
            .map_err(|_| "credential_store_unavailable".to_string())?;
        file.sync_all()
            .map_err(|_| "credential_store_unavailable".to_string())?;
    }
    #[cfg(not(unix))]
    std::fs::write(&temporary, serialized)
        .map_err(|_| "credential_store_unavailable".to_string())?;

    std::fs::rename(temporary, path).map_err(|_| "credential_store_unavailable".to_string())
}

pub fn validate_provider_id(provider_id: &str) -> Result<&str, &'static str> {
    let provider_id = provider_id.trim();
    if provider_id.is_empty()
        || provider_id.len() > 64
        || !provider_id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err("invalid_provider_id");
    }
    Ok(provider_id)
}

pub fn get(provider_id: &str) -> Result<Option<String>, String> {
    let provider_id = validate_provider_id(provider_id).map_err(str::to_string)?;
    Ok(read_store()?.remove(provider_id))
}

pub fn set(provider_id: &str, credential: &str) -> Result<(), String> {
    let provider_id = validate_provider_id(provider_id).map_err(str::to_string)?;
    if credential.trim().is_empty() || credential.len() > 16 * 1024 {
        return Err("invalid_provider_credential".to_string());
    }
    let mut credentials = read_store()?;
    credentials.insert(provider_id.to_string(), credential.trim().to_string());
    write_store(&credentials)
}

pub fn delete(provider_id: &str) -> Result<(), String> {
    let provider_id = validate_provider_id(provider_id).map_err(str::to_string)?;
    let mut credentials = read_store()?;
    credentials.remove(provider_id);
    write_store(&credentials)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_provider_identifiers_that_cannot_be_safe_map_keys() {
        for provider in ["", "OpenAI", "../openai", "openai/key"] {
            assert!(validate_provider_id(provider).is_err(), "{provider}");
        }
        assert_eq!(validate_provider_id("openai-compatible"), Ok("openai-compatible"));
    }
}
