//! Workspace skills catalogue + install for the stdio host.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

/// Skills live under IsanAgent's workspace state directory.
pub fn list_workspace_skills(workspace_root: &Path) -> Result<Value, String> {
    let skills_dir = workspace_skills_dir(workspace_root)?;
    let entries = match std::fs::read_dir(&skills_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(json!({ "skills": [] }));
        }
        Err(error) => return Err(error.to_string()),
    };

    let mut skills = entries
        .flatten()
        .filter_map(|entry| {
            let kind = entry.file_type().ok()?;
            if !kind.is_dir() {
                return None;
            }
            let name = entry.file_name().to_string_lossy().trim().to_string();
            if name.is_empty() || name.starts_with('.') {
                return None;
            }
            let description = std::fs::read_to_string(entry.path().join("SKILL.md"))
                .ok()
                .and_then(|text| skill_description(&text));
            Some(json!({
                "name": name,
                "description": description,
                "enabled": true,
            }))
        })
        .collect::<Vec<_>>();
    skills.sort_by(|a, b| {
        let an = a.get("name").and_then(Value::as_str).unwrap_or("");
        let bn = b.get("name").and_then(Value::as_str).unwrap_or("");
        an.to_lowercase().cmp(&bn.to_lowercase())
    });
    Ok(json!({ "skills": skills }))
}

/// Install skill(s) from a GitHub repo into IsanAgent's workspace skill directory.
///
/// Params mirror the Desktop Tauri command: `source` is owner/repo or a full
/// URL; optional `skill` installs a single named skill from that repo.
pub async fn install_workspace_skills(
    workspace_root: &Path,
    source: &str,
    skill: Option<&str>,
) -> Result<Value, String> {
    let repo = source.trim();
    if repo.is_empty() {
        return Err("A repository URL or owner/repo is required.".to_string());
    }
    let mut registry = isanagent::skills::SkillRegistry::new(workspace_skills_dir(workspace_root)?);
    let skill = skill.map(str::trim).filter(|s| !s.is_empty());
    let installed = registry.install_skills_from_repo(repo, skill).await?;
    if installed.is_empty() {
        return Err("No skills found in that repository.".to_string());
    }

    let listed = list_workspace_skills(workspace_root)?;
    let catalog = listed
        .get("skills")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut skills = installed
        .iter()
        .map(|name| {
            catalog
                .iter()
                .find(|row| row.get("name").and_then(Value::as_str) == Some(name.as_str()))
                .cloned()
                .unwrap_or_else(|| {
                    json!({
                        "name": name,
                        "description": Value::Null,
                        "enabled": true,
                    })
                })
        })
        .collect::<Vec<_>>();
    skills.sort_by(|a, b| {
        let an = a.get("name").and_then(Value::as_str).unwrap_or("");
        let bn = b.get("name").and_then(Value::as_str).unwrap_or("");
        an.to_lowercase().cmp(&bn.to_lowercase())
    });
    Ok(json!({
        "skills": skills,
        "installed": installed,
    }))
}

fn workspace_skills_dir(workspace_root: &Path) -> Result<PathBuf, String> {
    let state_dir = workspace_root.join(".isanagent");
    let state_dir = state_dir
        .to_str()
        .ok_or_else(|| "workspace path is not valid UTF-8".to_string())?;
    let workspace = isanagent::workspace::IsanagentWorkspace::new(Some(state_dir), None)?;
    Ok(workspace.skills_path())
}

fn skill_description(text: &str) -> Option<String> {
    for line in text.lines().take(40) {
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_prefix("description:") {
            let value = value.trim().trim_matches(['\'', '"']);
            if !value.is_empty() {
                return Some(value.chars().take(180).collect());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn lists_skills_from_workspace() {
        let dir = tempdir().unwrap();
        let skill_dir = dir
            .path()
            .join(".isanagent")
            .join("workspace")
            .join("skills")
            .join("demo");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\ndescription: Demo skill\n---\n",
        )
        .unwrap();
        let result = list_workspace_skills(dir.path()).unwrap();
        let skills = result["skills"].as_array().unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0]["name"], "demo");
        assert_eq!(skills[0]["description"], "Demo skill");
    }

    #[test]
    fn empty_when_missing_dir() {
        let dir = tempdir().unwrap();
        let result = list_workspace_skills(dir.path()).unwrap();
        assert_eq!(result["skills"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn install_rejects_empty_source() {
        let dir = tempdir().unwrap();
        let err = install_workspace_skills(dir.path(), "  ", None)
            .await
            .unwrap_err();
        assert!(err.contains("repository"), "{err}");
    }
}
