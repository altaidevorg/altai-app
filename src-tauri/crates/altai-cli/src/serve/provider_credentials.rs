//! Native-host provider credentials.
//!
//! Credentials are intentionally owned by the Rust stdio host.  They never
//! appear in protocol responses and callers can only observe whether a
//! provider is configured.  The Webview therefore cannot retrieve a stored
//! credential even when it asks the Extension Host to initiate a connection.

#[cfg(not(windows))]
use std::collections::HashMap;
#[cfg(not(windows))]
use std::path::PathBuf;

#[cfg(not(windows))]
const CREDENTIALS_FILE: &str = "agent-host-credentials.json";
#[cfg(windows)]
const CREDENTIAL_SERVICE: &str = "altai-agent-host";

#[cfg(not(windows))]
fn credentials_path() -> Result<PathBuf, String> {
    if let Some(directory) = std::env::var_os("ALTAI_CLI_CREDENTIALS_DIR") {
        return Ok(PathBuf::from(directory).join(CREDENTIALS_FILE));
    }
    dirs::data_local_dir()
        .map(|directory| directory.join("ALTAI").join(CREDENTIALS_FILE))
        .ok_or_else(|| "credential_store_unavailable".to_string())
}

#[cfg(not(windows))]
fn read_store() -> Result<HashMap<String, String>, String> {
    let path = credentials_path()?;
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let contents = std::fs::read(&path).map_err(|_| "credential_store_unavailable".to_string())?;
    serde_json::from_slice(&contents).map_err(|_| "credential_store_unavailable".to_string())
}

#[cfg(not(windows))]
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

#[cfg(not(windows))]
pub fn get(provider_id: &str) -> Result<Option<String>, String> {
    let provider_id = validate_provider_id(provider_id).map_err(str::to_string)?;
    Ok(read_store()?.remove(provider_id))
}

#[cfg(windows)]
pub fn get(provider_id: &str) -> Result<Option<String>, String> {
    let provider_id = validate_provider_id(provider_id).map_err(str::to_string)?;
    let entry = keyring::Entry::new(CREDENTIAL_SERVICE, provider_id)
        .map_err(|_| "credential_store_unavailable".to_string())?;
    match entry.get_password() {
        Ok(credential) => Ok(Some(credential)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(_) => Err("credential_store_unavailable".to_string()),
    }
}

#[cfg(not(windows))]
pub fn set(provider_id: &str, credential: &str) -> Result<(), String> {
    let provider_id = validate_provider_id(provider_id).map_err(str::to_string)?;
    if credential.trim().is_empty() || credential.len() > 16 * 1024 {
        return Err("invalid_provider_credential".to_string());
    }
    let mut credentials = read_store()?;
    credentials.insert(provider_id.to_string(), credential.trim().to_string());
    write_store(&credentials)
}

#[cfg(windows)]
pub fn set(provider_id: &str, credential: &str) -> Result<(), String> {
    let provider_id = validate_provider_id(provider_id).map_err(str::to_string)?;
    if credential.trim().is_empty() || credential.len() > 16 * 1024 {
        return Err("invalid_provider_credential".to_string());
    }
    keyring::Entry::new(CREDENTIAL_SERVICE, provider_id)
        .map_err(|_| "credential_store_unavailable".to_string())?
        .set_password(credential.trim())
        .map_err(|_| "credential_store_unavailable".to_string())
}

#[cfg(not(windows))]
pub fn delete(provider_id: &str) -> Result<(), String> {
    let provider_id = validate_provider_id(provider_id).map_err(str::to_string)?;
    let mut credentials = read_store()?;
    credentials.remove(provider_id);
    write_store(&credentials)
}

#[cfg(windows)]
pub fn delete(provider_id: &str) -> Result<(), String> {
    let provider_id = validate_provider_id(provider_id).map_err(str::to_string)?;
    let entry = keyring::Entry::new(CREDENTIAL_SERVICE, provider_id)
        .map_err(|_| "credential_store_unavailable".to_string())?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(_) => Err("credential_store_unavailable".to_string()),
    }
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
